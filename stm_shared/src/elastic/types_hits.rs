use serde::Deserialize;

/// Member of ESHitsCount
#[derive(Deserialize)]
pub struct ESHitsCountTotals {
    pub value: usize,
}

/// Member of ESHitsCount
#[derive(Deserialize)]
pub struct ESHitsCountHits {
    pub total: ESHitsCountTotals,
}
