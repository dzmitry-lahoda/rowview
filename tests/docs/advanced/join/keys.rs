//! Projection from map iterator keys and values.

use std::collections::{BTreeMap, HashMap};

#[test]
fn hash_map_projection_from_key_to_value() {
    struct Root {
        accounts: Vec<u32>,
        balances: HashMap<u32, i64>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = account_rows, axis = root.accounts)]
        #[joins(left = root.balances, as = balance, on(*axis = *balance.0))]
        struct AccountRow {
            #[from_axis(*axis)]
            account_id: u32,
            #[select(select = *balance.0)]
            balance_account_id: Option<u32>,
            #[select(select = *balance.1)]
            balance: Option<i64>,
        }
    }

    let rows = Root {
        accounts: vec![1, 2, 3],
        balances: HashMap::from([(1, 100), (3, -30)]),
    }
    .to_rows()
    .account_rows;

    assert_eq!(rows.len(), 3);
    assert_eq!(rows.account_id[0], 1);
    assert_eq!(rows.balance_account_id[0], Some(1));
    assert_eq!(rows.balance[0], Some(100));
    assert_eq!(rows.account_id[1], 2);
    assert_eq!(rows.balance_account_id[1], None);
    assert_eq!(rows.balance[1], None);
    assert_eq!(rows.account_id[2], 3);
    assert_eq!(rows.balance_account_id[2], Some(3));
    assert_eq!(rows.balance[2], Some(-30));
}

#[test]
fn btree_map_projection_from_key_to_value_with_multiple_joins() {
    struct Root {
        accounts: Vec<u32>,
        balances: BTreeMap<u32, i64>,
        fees: HashMap<u32, i16>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = account_rows, axis = root.accounts)]
        #[joins(left = root.balances, as = balance, on(*axis = *balance.0))]
        #[joins(left = root.fees, as = fee, on(*axis = *fee.0))]
        struct AccountRow {
            #[from_axis(*axis)]
            account_id: u32,
            #[select(select = *balance.0)]
            balance_account_id: Option<u32>,
            #[select(select = *balance.1)]
            balance: Option<i64>,
            #[select(select = *fee.0)]
            fee_account_id: Option<u32>,
            #[select(select = *fee.1)]
            fee: Option<i16>,
        }
    }

    let rows = Root {
        accounts: vec![1, 2, 3],
        balances: BTreeMap::from([(1, 100), (3, -30)]),
        fees: HashMap::from([(2, 7), (3, 9)]),
    }
    .to_rows()
    .account_rows;

    assert_eq!(rows.len(), 3);
    assert_eq!(rows.account_id[0], 1);
    assert_eq!(rows.balance_account_id[0], Some(1));
    assert_eq!(rows.balance[0], Some(100));
    assert_eq!(rows.fee_account_id[0], None);
    assert_eq!(rows.fee[0], None);
    assert_eq!(rows.account_id[1], 2);
    assert_eq!(rows.balance_account_id[1], None);
    assert_eq!(rows.balance[1], None);
    assert_eq!(rows.fee_account_id[1], Some(2));
    assert_eq!(rows.fee[1], Some(7));
    assert_eq!(rows.account_id[2], 3);
    assert_eq!(rows.balance_account_id[2], Some(3));
    assert_eq!(rows.balance[2], Some(-30));
    assert_eq!(rows.fee_account_id[2], Some(3));
    assert_eq!(rows.fee[2], Some(9));
}
