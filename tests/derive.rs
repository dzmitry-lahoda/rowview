type AccountId = u32;
type MarketId = u16;
type Balance = i64;
type TokenId = u16;
type Size = i64;
type OrderId = u64;
type FundingIndex = i128;
type ActionId = u64;
type Price = u64;

struct Taker {
    added_order: Optio<(OrderId, Size)>,
}

struct TradeOrPlace {
    market_id: MarketId,
  
    balances_final: Vec<(AccountId, (TokenId, Option<Balance>))>,
    pnl_delta: Vec<(AccountId, (TokenId, Balance))>,
    funding_index: Vec<AccountId, Option<FundingIndex>>,
}

struct TradedAccounts {
    cancelled_orders: Vec<Order>,
    pub fills: Vec<Fill>,
}

struct Action {}

struct Root {
    action_id: ActionId,
    action: &Action,
    receipt: &TradeOrPlace,
}

#[rowview::rows(root = Root)]
mod schema {
    #[rowset(name = orders, axis = orders)]
    struct OrderRow {
        #[copy(root.market_id)]
        market_id: u32,
        #[from_axis(axis[..].0)]
        account_id: u32,
        #[from_axis(axis[..].1.0)]
        order_id: u64,
        #[from_axis(axis[..].1.1)]
        order_size: i64,
    }

    #[rowset(name = Pnl, axis = Pnl)]
    struct Pnl {}

    // #[rowset(name = balances, axis = balances)]
    // struct BalanceRow {
    //     #[copy]
    //     account_id: Root.account_id,
    //     #[from_axis]
    //     asset: Root.balances[].asset,
    //     #[from_axis]
    //     free: Root.balances[].free,
    // }
}

#[test]
fn all() {
    let orders = vec![(1, (42, -500)), (2, (43, 100)), (1, (44, 400))];
    let root = TradeOrPlace {
        market_id: 13,
        cancelled_orders: orders,
    };

    let schema = root.to_rows();

    assert_eq!(schema.orders[0].market_id, 13);
    assert_eq!(schema.orders[0].account_id, 1);
    assert_eq!(schema.orders[0].order_id, 42);
    assert_eq!(schema.orders[0].order_size, -500);
}

// pub struct TradeOrPlaceReceipt {
//     pub base_trade_id: TradeId,
//     pub market_id: MarketId,
//     pub accounts: TradedAccountsResult,
//     pub market_price: Option<PositivePriceMantissa>,
//     pub client_order_id: Option<SenderTrackingId>,
//     pub sender_tracking_id: Option<SenderTrackingId>,
//     /// Please note that part of info is in `accounts` field.
//     pub taker: TakerResult,
// }

// pub struct TradedAccountsResult {
//     /// All changed positions, exact same as retained in state after action.
//     /// If None - no open position remained.
//     pub open: Vec<(AccountId, Option<OpenOrdersPosition>)>,
//     /// If None - position was closed.
//     pub perp_size: Vec<(AccountId, Option<PositionEntry>)>,
//     pub perp_funding: Vec<(AccountId, Option<FundingIndexMantissa>)>,
//     /// If there, was applied.
//     pub price_size_pnl: Vec<(AccountId, SidedBalance)>,
//     /// If there, was applied.
//     pub funding_index_pnl: Vec<(AccountId, SidedBalance)>,
//     /// Ordering is significant; in case of duplicates last occurrence is authoritative.
//     /// Exact balance as is.
//     pub balances: Vec<(AccountId, TokenId, Balance)>,
// }

pub struct Fill {
    pub order_final: Order,
    pub size_delta: Size,
}

pub struct Order {
    pub account_id: AccountId,
    pub order_id: OrderId,
    pub is_reduce_only: bool,
    pub price: Price,
    pub size: Size,
}

// pub struct OpenOrdersPosition {
//     ask_size: OrderSize,
//     bid_size: OrderSize,
//     orders_count: NonZero<u16>,
//     bids_quote_size: u128,
//     ask_quote_size: u128,
// }

// pub struct TakerResult {
//     pub side: Side,
//     pub taker_account_id: AccountId,
//     pub(crate) posted: Option<(OrderRecord, ViewAccountOrder)>,
// }
