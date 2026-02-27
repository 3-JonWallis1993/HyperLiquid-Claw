// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test, console2} from "forge-std/Test.sol";
import {HyperClawVault} from "../contracts/HyperClawVault.sol";
import {HyperClawRegistry} from "../contracts/HyperClawRegistry.sol";
import {ERC20Mock} from "@openzeppelin/contracts/mocks/token/ERC20Mock.sol";

contract HyperClawVaultTest is Test {
    HyperClawVault vault;
    HyperClawRegistry registry;
    ERC20Mock usdc;

    address owner = address(0xBEEF);
    address alice = address(0xA11CE);
    address bob   = address(0xB0B);

    uint256 constant INITIAL_USDC = 10_000e6; // 10k USDC

    function setUp() public {
        usdc = new ERC20Mock();
        vault = new HyperClawVault(address(usdc), owner);
        registry = new HyperClawRegistry(owner);

        // Fund users
        usdc.mint(alice, INITIAL_USDC);
        usdc.mint(bob,   INITIAL_USDC);
        usdc.mint(owner, INITIAL_USDC);

        vm.prank(alice);
        usdc.approve(address(vault), type(uint256).max);
        vm.prank(bob);
        usdc.approve(address(vault), type(uint256).max);
        vm.prank(owner);
        usdc.approve(address(vault), type(uint256).max);
    }

    // ── Vault tests ──────────────────────────────────────────────────────────

    function test_deposit_first_user_gets_1to1_shares() public {
        vm.prank(alice);
        vault.deposit(1000e6);

        assertEq(vault.shares(alice), 1000e6);
        assertEq(vault.totalShares(), 1000e6);
        assertEq(vault.totalAssets(), 1000e6);
    }

    function test_second_deposit_proportional_shares() public {
        vm.prank(alice);
        vault.deposit(1000e6);

        vm.prank(bob);
        vault.deposit(500e6);

        // Bob gets half as many shares (500/1000 * 1000 = 500)
        assertEq(vault.shares(bob), 500e6);
    }

    function test_withdraw_returns_proportional_usdc() public {
        vm.prank(alice);
        vault.deposit(1000e6);

        uint256 balBefore = usdc.balanceOf(alice);
        vm.prank(alice);
        vault.withdraw(0); // redeem all

        uint256 balAfter = usdc.balanceOf(alice);
        assertEq(balAfter - balBefore, 1000e6);
    }

    function test_profit_increases_nav_per_share() public {
        vm.prank(alice);
        vault.deposit(1000e6);

        // Simulate profit: owner bridges, earns 10%, returns 1100 USDC
        vm.prank(owner);
        vault.bridgeToL1(1000e6);

        vm.prank(owner);
        vault.receiveBridgeReturn(1100e6);

        // NAV per share should reflect 10% gain
        uint256 nav = vault.navPerShare();
        assertGt(nav, 1e6); // greater than 1 USDC per share
    }

    function test_revert_on_zero_deposit() public {
        vm.prank(alice);
        vm.expectRevert(HyperClawVault.ZeroAmount.selector);
        vault.deposit(0);
    }

    function test_revert_withdraw_insufficient_shares() public {
        vm.prank(alice);
        vm.expectRevert(HyperClawVault.InsufficientShares.selector);
        vault.withdraw(1);
    }

    function test_pause_blocks_deposits() public {
        vm.prank(owner);
        vault.setPaused(true);

        vm.prank(alice);
        vm.expectRevert(HyperClawVault.VaultPaused.selector);
        vault.deposit(100e6);
    }

    // ── Registry tests ───────────────────────────────────────────────────────

    function test_register_user() public {
        address hlAddr = address(0x1234);
        vm.prank(alice);
        registry.register(hlAddr, 10, 500, 2000, true);

        HyperClawRegistry.UserConfig memory cfg = registry.getConfig(alice);
        assertEq(cfg.hlAddress, hlAddr);
        assertEq(cfg.maxLeverage, 10);
        assertTrue(cfg.hedgeEnabled);
        assertTrue(cfg.active);
    }

    function test_is_authorised() public {
        address hlAddr = address(0xDEAD);
        vm.prank(alice);
        registry.register(hlAddr, 5, 300, 1500, false);

        assertTrue(registry.isAuthorised(alice, hlAddr));
        assertFalse(registry.isAuthorised(alice, address(0x9999)));
    }

    function test_revert_double_register() public {
        vm.prank(alice);
        registry.register(address(0x1), 5, 300, 1000, false);

        vm.prank(alice);
        vm.expectRevert(HyperClawRegistry.AlreadyRegistered.selector);
        registry.register(address(0x2), 5, 300, 1000, false);
    }

    function test_revert_invalid_leverage() public {
        vm.prank(alice);
        vm.expectRevert(HyperClawRegistry.InvalidLeverage.selector);
        registry.register(address(0x1), 51, 300, 1000, false); // > 50
    }

    function test_deactivate_removes_hl_mapping() public {
        address hlAddr = address(0xCAFE);
        vm.prank(alice);
        registry.register(hlAddr, 3, 200, 1000, false);

        vm.prank(alice);
        registry.deactivate();

        assertFalse(registry.getConfig(alice).active);
        assertEq(registry.hlToEvm(hlAddr), address(0));
    }
}
