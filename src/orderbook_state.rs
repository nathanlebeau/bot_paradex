use log::debug;
use paradex::structs::Level;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;

pub struct OrderBookState {
    // Bids
    pub first_bid: Arc<Mutex<Option<Level>>>,
    pub second_bid: Arc<Mutex<Option<Level>>>,
    pub third_bid: Arc<Mutex<Option<Level>>>,
    // Asks
    pub first_ask: Arc<Mutex<Option<Level>>>,
    // Notification
    pub notify: Arc<Notify>,
}

impl OrderBookState {
    pub fn new() -> Self {
        OrderBookState {
            first_bid: Arc::new(Mutex::new(None)),
            second_bid: Arc::new(Mutex::new(None)),
            third_bid: Arc::new(Mutex::new(None)),
            first_ask: Arc::new(Mutex::new(None)),
            notify: Arc::new(Notify::new()),
        }
    }

    // Fonction utilitaire pour cloner les références pour le callback
    pub fn clone_for_callback(&self) -> OrderBookStateCallbackClones {
        OrderBookStateCallbackClones {
            first_bid: Arc::clone(&self.first_bid),
            second_bid: Arc::clone(&self.second_bid),
            third_bid: Arc::clone(&self.third_bid),
            first_ask: Arc::clone(&self.first_ask),
            notify: Arc::clone(&self.notify),
        }
    }
}

// Structure temporaire pour passer les clones au callback
pub struct OrderBookStateCallbackClones {
    pub first_bid: Arc<Mutex<Option<Level>>>,
    pub second_bid: Arc<Mutex<Option<Level>>>,
    pub third_bid: Arc<Mutex<Option<Level>>>,
    pub first_ask: Arc<Mutex<Option<Level>>>,
    pub notify: Arc<Notify>,
}

pub fn extract_data_from_snapshot(
    ob_snapshot: &paradex::structs::OrderBook,
    clones: &OrderBookStateCallbackClones,
) {
    debug!("Snaphot OrderBook received. Extracting data...");

    let inserts = &ob_snapshot.inserts;

    // Bids
    if inserts.len() >= 1 {
        *clones.first_bid.lock().unwrap() = Some(inserts[0].clone());
    }
    if inserts.len() >= 2 {
        *clones.second_bid.lock().unwrap() = Some(inserts[1].clone());
    }
    if inserts.len() >= 3 {
        *clones.third_bid.lock().unwrap() = Some(inserts[2].clone());
    }

    // First ask
    if let Some(first_ask_level) = inserts
        .iter()
        .find(|level| level.side == paradex::structs::Side::SELL)
    {
        *clones.first_ask.lock().unwrap() = Some(first_ask_level.clone());
    }

    // Notification to unsubscribe
    clones.notify.notify_one();
    debug!("Data extracted and signal sent.");
}
