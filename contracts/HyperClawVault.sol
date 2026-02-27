// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/security/ReentrancyGuard.sol";

/// @title  HyperClawVault
/// @notice Custodial USDC vault that bridges funds to Hyperliquid L1 for
///         automated perp trading by the HyperLiquid-Claw bot.
///
///         Flow:
///           1. Users deposit USDC → receive proportional vault shares
///           2. Owner (bot) signals a bridge to Hyperliquid L1
///           3. Trading profits flow back via withdrawBridge
///           4. Users redeem shares for USDC + accumulated profit
///
/// @dev    Production deployment should replace the bridge logic with the
///         official Hyperliquid bridge contract call. This contract is
///         intentionally minimal for clarity and auditability.
contract HyperClawVault is Ownable, ReentrancyGuard {
    using SafeERC20 for IERC20;

    // ─── State ────────────────────────────────────────────────────────────────

    IERC20 public immutable usdc;

    /// ERC-20-like share accounting (non-transferable)
    mapping(address => uint256) public shares;
    uint256 public totalShares;
    uint256 public totalAssets;

    /// Funds currently deployed to Hyperliquid L1
    uint256 public bridgedAmount;

    /// Emergency pause flag
    bool public paused;

    // ─── Events ───────────────────────────────────────────────────────────────

    event Deposited(address indexed user, uint256 usdcAmount, uint256 sharesIssued);
    event Withdrawn(address indexed user, uint256 sharesRedeemed, uint256 usdcReturned);
    event BridgedToL1(uint256 amount, uint256 timestamp);
    event BridgeReturnReceived(uint256 amount, uint256 profit);
    event EmergencyPause(bool paused);

    // ─── Errors ───────────────────────────────────────────────────────────────

    error ZeroAmount();
    error InsufficientShares();
    error VaultPaused();
    error NotEnoughLiquidity();
    error AlreadyBridged();

    // ─── Constructor ──────────────────────────────────────────────────────────

    constructor(address _usdc, address _owner) Ownable(_owner) {
        usdc = IERC20(_usdc);
    }

    // ─── User actions ─────────────────────────────────────────────────────────

    /// @notice Deposit USDC and receive proportional vault shares.
    /// @param amount USDC amount (6 decimals) to deposit.
    function deposit(uint256 amount) external nonReentrant whenNotPaused {
        if (amount == 0) revert ZeroAmount();

        uint256 issued = _calculateShares(amount);
        shares[msg.sender] += issued;
        totalShares += issued;
        totalAssets += amount;

        usdc.safeTransferFrom(msg.sender, address(this), amount);
        emit Deposited(msg.sender, amount, issued);
    }

    /// @notice Redeem shares for USDC. Proportional to vault NAV.
    /// @param shareAmount Number of shares to redeem (0 = redeem all).
    function withdraw(uint256 shareAmount) external nonReentrant whenNotPaused {
        uint256 toRedeem = shareAmount == 0 ? shares[msg.sender] : shareAmount;
        if (toRedeem == 0 || shares[msg.sender] < toRedeem) revert InsufficientShares();

        uint256 usdcOut = _calculateRedemption(toRedeem);
        if (usdcOut > usdc.balanceOf(address(this))) revert NotEnoughLiquidity();

        shares[msg.sender] -= toRedeem;
        totalShares -= toRedeem;
        totalAssets -= usdcOut;

        usdc.safeTransfer(msg.sender, usdcOut);
        emit Withdrawn(msg.sender, toRedeem, usdcOut);
    }

    // ─── Owner (bot) actions ──────────────────────────────────────────────────

    /// @notice Bridge USDC to Hyperliquid L1 for trading.
    /// @dev    In production, replace this with the HL bridge contract call.
    function bridgeToL1(uint256 amount) external onlyOwner whenNotPaused {
        if (bridgedAmount > 0) revert AlreadyBridged();
        if (amount > usdc.balanceOf(address(this))) revert NotEnoughLiquidity();

        bridgedAmount = amount;
        // TODO: call Hyperliquid bridge contract
        // IHyperliquidBridge(BRIDGE).deposit(amount);
        usdc.safeTransfer(owner(), amount); // placeholder: transfer to bot wallet

        emit BridgedToL1(amount, block.timestamp);
    }

    /// @notice Receive trading proceeds back from Hyperliquid L1.
    /// @param returnAmount Total USDC being returned (principal + profit).
    function receiveBridgeReturn(uint256 returnAmount) external onlyOwner {
        if (returnAmount == 0) revert ZeroAmount();

        uint256 profit = returnAmount > bridgedAmount ? returnAmount - bridgedAmount : 0;
        totalAssets = totalAssets - bridgedAmount + returnAmount;
        bridgedAmount = 0;

        usdc.safeTransferFrom(msg.sender, address(this), returnAmount);
        emit BridgeReturnReceived(returnAmount, profit);
    }

    /// @notice Emergency pause/unpause.
    function setPaused(bool _paused) external onlyOwner {
        paused = _paused;
        emit EmergencyPause(_paused);
    }

    // ─── Views ────────────────────────────────────────────────────────────────

    /// @notice Current NAV per share (in USDC micro-units, 6 decimals).
    function navPerShare() external view returns (uint256) {
        if (totalShares == 0) return 1e6; // 1 USDC initial
        return (totalAssets * 1e18) / totalShares;
    }

    /// @notice USDC value of a given number of shares.
    function sharesValue(address user) external view returns (uint256) {
        return _calculateRedemption(shares[user]);
    }

    // ─── Internal ─────────────────────────────────────────────────────────────

    function _calculateShares(uint256 usdcAmount) internal view returns (uint256) {
        if (totalShares == 0 || totalAssets == 0) {
            return usdcAmount; // 1:1 on first deposit
        }
        return (usdcAmount * totalShares) / totalAssets;
    }

    function _calculateRedemption(uint256 shareAmount) internal view returns (uint256) {
        if (totalShares == 0) return 0;
        return (shareAmount * totalAssets) / totalShares;
    }

    modifier whenNotPaused() {
        if (paused) revert VaultPaused();
        _;
    }
}
