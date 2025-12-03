use log::{debug, error, info};

use paradex::{rest::Client, structs, url::URL, ws};
use structs::{
    ModifyOrderRequest, OrderInstruction, OrderRequest, OrderType, OrderUpdate, OrderUpdates,
    Position, PositionStatus, Side,
};
use ws::{Channel, Message, WebsocketManager};

use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use std::time::Duration;

mod orderbook_state;
use orderbook_state::OrderBookState;

const REFRESH_TIME_SEC: u64 = 10;
const SIZE_MULTIPLIER_BIDDING_MARGIN: i32 = 5;
const STEP_SIZE: f64 = 0.1;
const MAX_SPREAD_PRICE: i32 = 5;

async fn run_orderbook_subscription(
    manager: &mut WebsocketManager,
    market_symbol: String,
    state: &OrderBookState,
) -> Result<(), Box<dyn std::error::Error>> {
    let clones = state.clone_for_callback();
    // Get the order book using public manager
    debug!("The market of the order is: {:?}", market_symbol);
    let orderbook_id = manager
        .subscribe(
            Channel::OrderBook {
                market_symbol: market_symbol.clone(),
                channel_name: None,
                refresh_rate: "50ms".into(),
                price_tick: None,
            },
            Box::new(move |message| {
                debug!("Received message!");
                match message {
                    Message::OrderBook(ob_snapshot) => {
                        orderbook_state::extract_data_from_snapshot(&ob_snapshot, &clones);
                    }
                    // ignore other variants for the moment
                    _ => {}
                }
            }),
        )
        .await
        .unwrap();
    // wait for message
    debug!("Waiting for OrderBook snapshot notification...");
    state.notify.notified().await;
    // then unsubscribe
    debug!("Notification received! Unsubscribing...");
    manager.unsubscribe(orderbook_id).await.unwrap();

    Ok(())
}

async fn adjust_order(
    client_private: &mut Client,
    order_id: String,
    order_market: String,
    order_size: Decimal,
    new_price: Option<Decimal>,
) {
    let modify_request = ModifyOrderRequest {
        id: order_id,
        market: order_market,
        price: new_price,
        side: Side::BUY,
        size: order_size,
        order_type: OrderType::LIMIT,
    };
    info!("Sending modify order {modify_request:?}");
    let result = client_private.modify_order(modify_request).await.unwrap();
    info!("Modify order result {result:?}");
}

async fn determine_new_bid_price(
    order: &OrderUpdate,
    state: &OrderBookState,
    step_size: Decimal,
) -> Option<Decimal> {
    // Keep old price
    let current_price = order.price.unwrap_or_default();
    let mut new_price = current_price;

    if let Some(bid) = state.first_bid.lock().unwrap().as_ref() {
        debug!("First bid written by callback: {:?}", bid);
        if let Some(decimal_price) = order.price {
            // compare two prices as decimals
            if let Some(bid_price_decimal) = Decimal::from_f64(bid.price) {
                if decimal_price == bid_price_decimal {
                    debug!("We are first bid!");
                    // Check if we are + step_size from second bid or not alone at first bid
                    if let Some(bid_size_decimal) = Decimal::from_f64(bid.size) {
                        if order.size == bid_size_decimal {
                            debug!(
                                "We are first bid alone at the top! Checking if need to re-price."
                            );
                            if let Some(sec_bid) = state.second_bid.lock().unwrap().as_ref() {
                                if let Some(sec_bid_size_decimal) = Decimal::from_f64(sec_bid.price)
                                {
                                    if new_price - sec_bid_size_decimal > step_size {
                                        debug!("Need re-adjust!");
                                        new_price = sec_bid_size_decimal + step_size;
                                    }
                                }
                            }
                        } else {
                            debug!("We are not alone at the top. Checking if can go first bid alone.");
                            if let Some(ask) = state.first_ask.lock().unwrap().as_ref() {
                                if let Some(ask_price_decimal) = Decimal::from_f64(ask.price) {
                                    if ask_price_decimal != bid_price_decimal + step_size {
                                        new_price = bid_price_decimal + step_size;
                                    }
                                }
                            }
                        }
                    }
                } else {
                    debug!("We are NOT first bid!");
                    new_price = bid_price_decimal;
                }
            } else {
                error!("cannot convert f64 into decimal");
            }
        }
    }
    Some(new_price)
}

async fn check_liquidity_and_cancel_if_low(
    client_private: &mut Client,
    order: OrderUpdate,
    state: &OrderBookState,
    new_price: Option<Decimal>,
) {
    // Take the global size of first 3 bids
    let bid1_opt = state.first_bid.lock().unwrap();
    let bid2_opt = state.second_bid.lock().unwrap();
    let bid3_opt = state.third_bid.lock().unwrap();
    if let (Some(bid1), Some(bid2), Some(bid3)) =
        (bid1_opt.as_ref(), bid2_opt.as_ref(), bid3_opt.as_ref())
    {
        if let Some(price_to_adjust) = new_price {
            let mut glob_size: f64 = 0.0;
            // Max spread is MAX_SPREAD_PRICE
            let max_diff_decimal = Decimal::from_i32(MAX_SPREAD_PRICE).unwrap_or_default();
            // Bid 1
            if let Some(bid1_price_decimal) = Decimal::from_f64(bid1.price) {
                if price_to_adjust - bid1_price_decimal < max_diff_decimal {
                    glob_size += bid1.size;
                }
            }
            // Bid 2
            if let Some(bid2_price_decimal) = Decimal::from_f64(bid2.price) {
                if price_to_adjust - bid2_price_decimal < max_diff_decimal {
                    glob_size += bid2.size;
                }
            }
            // Bid 3
            if let Some(bid3_price_decimal) = Decimal::from_f64(bid3.price) {
                if price_to_adjust - bid3_price_decimal < max_diff_decimal {
                    glob_size += bid3.size;
                }
            }
            debug!("Global used size: {:?}", glob_size);

            if let Some(glob_size_decimal) = Decimal::from_f64(glob_size) {
                if let Some(decimal_multiplier) = Decimal::from_i32(SIZE_MULTIPLIER_BIDDING_MARGIN)
                {
                    if glob_size_decimal < order.size * decimal_multiplier {
                        // cancel order.
                        let result = client_private.cancel_order(order.id.clone()).await;
                        info!("Cancelling order result {result:?}");
                    }
                }
            }
        }
    }
}

async fn process_option_open_orders(
    client_private: &mut Client,
    manager: &mut WebsocketManager,
    orders: OrderUpdates,
) {
    for order in orders.results {
        if !order.market.contains("-PERP") {
            let state = OrderBookState::new();

            if let Err(e) = run_orderbook_subscription(manager, order.market.clone(), &state).await
            {
                debug!("Subscription failed for market {}: {}", order.market, e);
                continue; // go for next order
            }

            // Process data retrieved with callback

            // 1) Are we first bid with good margin?
            let step_size = Decimal::from_f64(STEP_SIZE).unwrap_or_default();
            let new_price = determine_new_bid_price(&order, &state, step_size).await;

            // 2) Modify order if necessary
            if new_price != order.price {
                adjust_order(
                    client_private,
                    order.id.clone(),
                    order.market.clone(),
                    order.size,
                    new_price,
                )
                .await;
            }

            // 3) Is there sufficient size below our bid? Cancel order if that's not the case
            check_liquidity_and_cancel_if_low(client_private, order, &state, new_price).await;
        }
    }
}

pub async fn run_backend_logic() {
    // Log
    simple_logger::init_with_level(log::Level::Info).unwrap();
    
    // Public manager for WS
    let mut manager = WebsocketManager::new(URL::Production, None).await;
    // Private client for REST api
    let url = URL::Production;
    // Read key from env variable PARADEX_L2_KEY
    let l2_private_key_hex_str = std::env::var("PARADEX_L2_KEY").ok();
    let mut client_private = Client::new(url, l2_private_key_hex_str).await.unwrap();

    loop {
        // Any Option open positions? Cancel order of same marke + sell market
        let positions = client_private.positions().await;
        match positions {
            Ok(positions) => {
                let open_option_positions: Vec<Position> = positions
                    .results
                    .clone() // clone and consume values
                    .into_iter()
                    .filter(|position| {
                        position.status == PositionStatus::OPEN
                            && !position.market.contains("-PERP")
                    })
                    .collect();
                info!(
                    "Nbr of Option open positions: {:?}",
                    open_option_positions.len()
                );
                if open_option_positions.len() >= 1 {
                    for position in open_option_positions {
                        // cancel remaining order in this market
                        let result = client_private
                            .cancel_all_orders_for_market(position.market.clone())
                            .await;
                        info!("Cancelling order result {result:?}");
                        // Sell position
                        let order_request = OrderRequest {
                            instruction: OrderInstruction::GTC,
                            market: position.market.clone(),
                            price: None,
                            side: Side::SELL,
                            size: Decimal::from_f64(position.size).unwrap(),
                            order_type: OrderType::MARKET,
                            client_id: Some("order_sent_using_rust_api".into()),
                            flags: vec![],
                            recv_window: None,
                            stp: None,
                            trigger_price: None,
                        };
                        info!("Sending order {order_request:?}");
                        let result = client_private.create_order(order_request).await.unwrap();
                        info!("Sell order result {result:?}");
                    }
                }
            }
            Err(err) => {
                error!("Failed to fetch positions: {}", err);
            }
        }

        // For each open orders:
        // - go to first bid + STEP_SIZE margin if possible (depends of first ask)
        // - check if the below orders have enough size to absorb massive instant sell
        let orders = client_private.open_orders().await;
        match orders {
            Ok(orders) => {
                info!("Nbr of open orders: {:?}", orders.results.len());
                if orders.results.len() >= 1 {
                    process_option_open_orders(&mut client_private, &mut manager, orders).await;
                } else {
                    manager.stop().await.unwrap();
                    break;
                }
            }
            Err(err) => {
                error!("Failed to fetch orders: {}", err);
            }
        }

        tokio::time::sleep(Duration::from_secs(REFRESH_TIME_SEC)).await;
    }
}
