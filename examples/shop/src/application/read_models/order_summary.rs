use serde::Serialize;

/// What the actor sees on the screen: plain fields, no behavior, no invariants.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OrderSummary {
    pub product: String,
    pub quantity: u32,
    pub total: u64,
    pub status: String,
}
