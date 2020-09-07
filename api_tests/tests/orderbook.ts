/**
 * @ledger ethereum
 * @ledger bitcoin
 */

import { twoActorTest } from "../src/actor_test";
import { sleep } from "../src/utils";
import { Position } from "../src/cnd/payload";

describe("orderbook", () => {
    it(
        "herc20-hbit",
        twoActorTest(async ({ alice, bob }) => {
            await alice.connect(bob);

            await alice.makeBtcDaiOrder(Position.Buy, 0.2, 9000);
            await bob.makeBtcDaiOrder(Position.Sell, 0.2, 9000);

            await Promise.all([alice.waitForSwap(), bob.waitForSwap()]);

            await alice.assertAndExecuteNextAction("deploy");
            await alice.assertAndExecuteNextAction("fund");

            await bob.assertAndExecuteNextAction("fund");

            await alice.assertAndExecuteNextAction("redeem");
            await bob.assertAndExecuteNextAction("redeem");

            // Wait until the wallet sees the new balance.
            await sleep(2000);

            await alice.assertBalancesAfterSwap();
            await bob.assertBalancesAfterSwap();
        })
    );

    it(
        "hbit-herc20",
        twoActorTest(async ({ alice, bob }) => {
            await alice.connect(bob);

            await alice.makeBtcDaiOrder(Position.Sell, 0.2, 9000);
            await bob.makeBtcDaiOrder(Position.Buy, 0.2, 9000);

            await Promise.all([alice.waitForSwap(), bob.waitForSwap()]);

            await alice.assertAndExecuteNextAction("fund");

            await bob.assertAndExecuteNextAction("deploy");
            await bob.assertAndExecuteNextAction("fund");

            await alice.assertAndExecuteNextAction("redeem");
            await bob.assertAndExecuteNextAction("redeem");

            // Wait until the wallet sees the new balance.
            await sleep(2000);

            await alice.assertBalancesAfterSwap();
            await bob.assertBalancesAfterSwap();
        })
    );
});
