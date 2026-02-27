// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";

/// @title  HyperClawRegistry
/// @notice On-chain registry that maps Hyperliquid L1 addresses to EVM addresses
///         and stores per-user strategy configuration.
///
///         This allows HyperLiquid-Claw to verify that a given HL address is
///         authorised to act on behalf of a vault depositor, and to store
///         immutable strategy parameters on-chain for transparency.
contract HyperClawRegistry is Ownable {

    // ─── Structs ──────────────────────────────────────────────────────────────

    struct UserConfig {
        /// Hyperliquid L1 wallet address (20 bytes, same key space as EVM)
        address hlAddress;
        /// Maximum leverage allowed for this user's strategies (1–50)
        uint8 maxLeverage;
        /// Stop-loss threshold in basis points (e.g. 500 = 5%)
        uint16 stopLossBps;
        /// Take-profit threshold in basis points (e.g. 2000 = 20%)
        uint16 takeProfitBps;
        /// Whether the user has enabled automated hedging
        bool hedgeEnabled;
        /// Registration timestamp
        uint64 registeredAt;
        /// Active flag
        bool active;
    }

    // ─── State ────────────────────────────────────────────────────────────────

    mapping(address => UserConfig) public configs;
    mapping(address => address) public hlToEvm; // hlAddress → evmAddress
    address[] public registeredUsers;

    // ─── Events ───────────────────────────────────────────────────────────────

    event UserRegistered(address indexed evmAddr, address indexed hlAddr, uint64 timestamp);
    event UserUpdated(address indexed evmAddr);
    event UserDeactivated(address indexed evmAddr);

    // ─── Errors ───────────────────────────────────────────────────────────────

    error AlreadyRegistered();
    error NotRegistered();
    error InvalidLeverage();
    error InvalidAddress();
    error HlAddressTaken();

    // ─── Constructor ──────────────────────────────────────────────────────────

    constructor(address _owner) Ownable(_owner) {}

    // ─── User actions ─────────────────────────────────────────────────────────

    /// @notice Register a Hyperliquid address and strategy config.
    /// @param hlAddress    Your Hyperliquid L1 wallet address.
    /// @param maxLeverage  Maximum leverage (1–50).
    /// @param stopLossBps  Stop-loss in bps (100 = 1%).
    /// @param takeProfitBps Take-profit in bps.
    /// @param hedgeEnabled  Enable automated hedge discovery.
    function register(
        address hlAddress,
        uint8 maxLeverage,
        uint16 stopLossBps,
        uint16 takeProfitBps,
        bool hedgeEnabled
    ) external {
        if (configs[msg.sender].active) revert AlreadyRegistered();
        if (hlAddress == address(0)) revert InvalidAddress();
        if (hlToEvm[hlAddress] != address(0)) revert HlAddressTaken();
        if (maxLeverage == 0 || maxLeverage > 50) revert InvalidLeverage();

        configs[msg.sender] = UserConfig({
            hlAddress: hlAddress,
            maxLeverage: maxLeverage,
            stopLossBps: stopLossBps,
            takeProfitBps: takeProfitBps,
            hedgeEnabled: hedgeEnabled,
            registeredAt: uint64(block.timestamp),
            active: true
        });

        hlToEvm[hlAddress] = msg.sender;
        registeredUsers.push(msg.sender);

        emit UserRegistered(msg.sender, hlAddress, uint64(block.timestamp));
    }

    /// @notice Update strategy parameters.
    function updateConfig(
        uint8 maxLeverage,
        uint16 stopLossBps,
        uint16 takeProfitBps,
        bool hedgeEnabled
    ) external {
        if (!configs[msg.sender].active) revert NotRegistered();
        if (maxLeverage == 0 || maxLeverage > 50) revert InvalidLeverage();

        UserConfig storage cfg = configs[msg.sender];
        cfg.maxLeverage = maxLeverage;
        cfg.stopLossBps = stopLossBps;
        cfg.takeProfitBps = takeProfitBps;
        cfg.hedgeEnabled = hedgeEnabled;

        emit UserUpdated(msg.sender);
    }

    /// @notice Deactivate registration.
    function deactivate() external {
        UserConfig storage cfg = configs[msg.sender];
        if (!cfg.active) revert NotRegistered();
        hlToEvm[cfg.hlAddress] = address(0);
        cfg.active = false;
        emit UserDeactivated(msg.sender);
    }

    // ─── Views ────────────────────────────────────────────────────────────────

    /// @notice Verify that an HL address is authorised for an EVM address.
    function isAuthorised(address evmAddr, address hlAddr) external view returns (bool) {
        UserConfig storage cfg = configs[evmAddr];
        return cfg.active && cfg.hlAddress == hlAddr;
    }

    function getConfig(address user) external view returns (UserConfig memory) {
        return configs[user];
    }

    function totalUsers() external view returns (uint256) {
        return registeredUsers.length;
    }
}
