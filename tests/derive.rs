


#[derive(rowview::RowView)]
struct Root {
    market_id: u32,
    orders: Vec<(u32, (u64, i64))>,
}


#[derive(rowview::RowView)]
#[rows(root = Root)]
mod schema {
    #[rowset(name = "orders", axis = "orders")]
    struct OrderRow {
        #[copy]
        market_id: Root.market_id,
        #[from_axis]
        account_id: Root.orders[].0
        #[from_axis]
        order_id: Root.orders[].1.0,
        #[from_axis]
        order_size: Root.orders[].1.1,
    }
}

//     #[rowset(name = "balances", axis = "balances")]
//     struct BalanceRow {
//         #[copy]
//         account_id: Root.account_id,
//         #[from_axis]
//         asset: Root.balances[].asset,
//         #[from_axis]
//         free: Root.balances[].free,
//     }
// }

#[test]
fn all() {
    let orders = vec![
        (1, (42,-500)),
        (2, (43, 100)),
        (1, (44, 400)),
    ];
    let root = Root {
        market_id: 13,
        orders,
    };

    let schema = orders.to_rows();

    assert_eq!(schema.orders[0].market_id,  13);
    assert_eq!(schema.orders[0].account_id,  1);
    assert_eq!(schema.orders[0].order_id,  42);
    assert_eq!(schema.orders[0].order_size,  -500);
}