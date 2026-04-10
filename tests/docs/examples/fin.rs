
#![allow(dead_code)]

use std::num::NonZero;

type AccountId = u32;
type MarketId = u16;
type Balance = i64;
type TokenId = u16;
type Size = i64;
type OpenSize = u64;
type OrderId = u64;
type FundingIndex = i128;
type ActionId = u64;
type Price = u64;
type Side = bool;

struct Taker {
    added_order: Option<(OrderId, Size)>,
    pub account_id: AccountId,
    pub client_order_id: Option<u64>,
}

struct TradeOrPlace {
    market_id: MarketId,
    pub trade_id_base: u64,
    pub taker: Taker,
    pub accounts: TradedAccounts,
    pub market_price: Option<Price>,
}

struct TradedAccounts {
    cancelled_orders: Vec<Order>,
    pub fills: Vec<Fill>,

    open_final: Vec<(AccountId, Option<Open>)>,
    balances_final: Vec<(AccountId, (TokenId, Option<Balance>))>,
    pnl_delta: Vec<(AccountId, (TokenId, Balance))>,
    funding_index_final: Vec<(AccountId, Option<FundingIndex>)>,
}

struct Action {
    pub side: Side,
    pub client_order_id: Option<u64>,
}

struct Root {
    action_id: ActionId,
    action: Action,
    receipt: TradeOrPlace,
}

#[rowview::rows(root = Root)]
mod schema {
    #[rowset(name = orders, axis = root.receipt.accounts.cancelled_orders)]
    struct OrderRow {
        #[copy(root.receipt.market_id)]
        market_id: u16,
        #[from_axis(axis.account_id)]
        account_id: u32,
        #[from_axis(axis.order_id)]
        order_id: u64,
        #[from_axis(axis.size)]
        order_size: i64,
    }

    // #[rowset(name = Pnl, axis = Pnl)]
    // struct Pnl {}

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
    let orders = vec![(1, (42, -500)), (2, (43, 100)), (1, (44, 400))]
        .into_iter()
        .map(|x| Order {
            account_id: x.0,
            order_id: x.1.0,
            is_reduce_only: false,
            price: 666_u64,
            size: x.1.1,
        }).collect();
    let receipt = TradeOrPlace {
        trade_id_base: 42,
        taker: Taker {
            added_order: None,
            account_id: 1,
            client_order_id: 13.into(),
        },
        accounts: TradedAccounts {
            cancelled_orders: orders,
            fills: vec![],
            open_final: vec![],
            balances_final: vec![(1, (13_u16, Some(1000))), (3, (13_u16, Some(2000)))],
            pnl_delta: vec![],
            funding_index_final: vec![],
        },
        market_price: Some(666_u64),
        market_id: 13,
    };
    let action = Action {
        side: false,
        client_order_id: Some(13),
    };
    let root = Root {
        action_id: 7,
        action,
        receipt,
    };

    let schema = root.to_rows();

    assert_eq!(schema.orders[0].market_id, 13);
    assert_eq!(schema.orders[0].account_id, 1);
    assert_eq!(schema.orders[0].order_id, 42);
    assert_eq!(schema.orders[0].order_size, -500);
}

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

pub struct Open {
    ask_size: OpenSize,
    bid_size: OpenSize,
    orders_count: NonZero<u16>,
    bids_quote_size: OpenSize,
    ask_quote_size: OpenSize,
}
