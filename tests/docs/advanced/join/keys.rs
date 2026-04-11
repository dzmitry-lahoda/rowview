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
        #[joins(left = root.balances, as = balance, on = *axis == *balance.0)]
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
    assert_eq!(rows[0].account_id, 1);
    assert_eq!(rows[0].balance_account_id, Some(1));
    assert_eq!(rows[0].balance, Some(100));
    assert_eq!(rows[1].account_id, 2);
    assert_eq!(rows[1].balance_account_id, None);
    assert_eq!(rows[1].balance, None);
    assert_eq!(rows[2].account_id, 3);
    assert_eq!(rows[2].balance_account_id, Some(3));
    assert_eq!(rows[2].balance, Some(-30));
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
        #[joins(left = root.balances, as = balance, on = *axis == *balance.0)]
        #[joins(left = root.fees, as = fee, on = *axis == *fee.0)]
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
    assert_eq!(rows[0].account_id, 1);
    assert_eq!(rows[0].balance_account_id, Some(1));
    assert_eq!(rows[0].balance, Some(100));
    assert_eq!(rows[0].fee_account_id, None);
    assert_eq!(rows[0].fee, None);
    assert_eq!(rows[1].account_id, 2);
    assert_eq!(rows[1].balance_account_id, None);
    assert_eq!(rows[1].balance, None);
    assert_eq!(rows[1].fee_account_id, Some(2));
    assert_eq!(rows[1].fee, Some(7));
    assert_eq!(rows[2].account_id, 3);
    assert_eq!(rows[2].balance_account_id, Some(3));
    assert_eq!(rows[2].balance, Some(-30));
    assert_eq!(rows[2].fee_account_id, Some(3));
    assert_eq!(rows[2].fee, Some(9));
}
