#[derive(rowview::RowView)]
struct Root {
    market_id: u32,
    orders: Vec<(u32, (u64, i64))>,
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

    let schema = root.to_rows();

    assert_eq!(schema.orders[0].market_id,  13);
    assert_eq!(schema.orders[0].account_id,  1);
    assert_eq!(schema.orders[0].order_id,  42);
    assert_eq!(schema.orders[0].order_size,  -500);
}
