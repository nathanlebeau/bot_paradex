# BOT PARADEX

This bot aims to farm Perp Options volume on Paradex (DEX) [https://github.com/tradeparadex] spending the 
less money.

## Financial Risk Warning / Disclaimer
This software is provided "as is" under the MIT License. It is an experimental algorithmic trading tool and DOES NOT constitute financial advice or an invitation to trade.

This bot utilizes the open-source community crate paradex-rs: [https://github.com/snow-avocado/paradex-rs] for interaction with the Paradex platform.

By using this software, you acknowledge and agree to the following:

- Risk of Loss: trading involves significant risk. You can lose all or more than your initial capital. Only risk capital that you are prepared to lose.

- Third-Party Crate Risk: the author does not guarantee the security, stability, or reliability of the paradex-rs crate or the underlying Paradex platform. Any issues arising from the use of this dependency are under the sole responsibility of the user.

- No Guarantee: the performance shown in any backtests or examples is not indicative of future results. The author makes no guarantee of profit or that the software will perform as expected.

- User Responsibility: you are solely responsible for all trading decisions, tax reports, and regulatory compliance in your jurisdiction.

- Limitation of Liability: the author is not responsible for any financial loss, damage, or other liability resulting directly or indirectly from the use or inability to use this software.

Use this software at your own risk.

## Prerequisites
Cargo and Rust installed.

## Usage
Designed for daemon usage. Each **REFRESH_TIME_SEC** seconds the bot:
    
1) Fetch open positions, cancel order of same markets then sell market.
2) Fetch open orders and:
   - go first bid with price + **STEP_SIZE** to second bid or with other first bid (revise down if      
     necessary)
   - check that there is a sufficient bidding size (sufficient = **SIZE_MULTIPLIER_BIDDING_MARGIN** *
     my_size) right above (right above = with spread of price inferior than **MAX_SPREAD_PRICE**) so 
     we can market sell with low price difference once we are filled. If the size is not sufficient, 
     order is simply cancelled. 

## Getting started
Define your L2 Paradex private key for example with command export PARADEX_L2_KEY=<my_hex_private_key> if you are on Linux system.

Then launch the program with "cargo run --bin app".

To have Debug log: modify ```let logger = Logger::with_level(log_sender, LogLevel::Info);``` 

into

```let logger = Logger::with_level(log_sender, LogLevel::Debug);```
