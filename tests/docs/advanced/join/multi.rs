//! If any axis exists, try to join with another axis.
// Multi-hop joins need dependency semantics:
// Given axes x, y, and a:
// - if x and y exist, y must exist for x
// - if y exists, a must exist for y
// - otherwise the whole chain is optional
// #[join_chain(option, x -> y must -> a must, select = a.value)]

type AccountId = u32;
type Balance = i64;
type NonZeroSidedSize = i64;
type FundingIndexMantissa = i64;
type PositiveQuoteSize = i64;

#[test]
fn account_pnl_rows_join_multiple_optional_sources() {
    struct Position {
        base: NonZeroSidedSize,
        funding_index: FundingIndexMantissa,
        quote: PositiveQuoteSize,
    }

    struct Root {
        accounts: Vec<AccountId>,
        price_size_pnl: Vec<(AccountId, Balance)>,
        funding_index_pnl: Vec<(AccountId, Balance)>,
        perp_size: Vec<(AccountId, NonZeroSidedSize)>,
        perp_funding: Vec<(AccountId, FundingIndexMantissa)>,
        positions: Vec<(AccountId, Position)>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        use super::{
            AccountId, Balance, FundingIndexMantissa, NonZeroSidedSize, PositiveQuoteSize,
        };

        #[rowset(name = account_pnls, axis = root.accounts)]
        #[joins(left = root.price_size_pnl[..], as = realized, on = *axis == realized.0)]
        #[joins(left = root.funding_index_pnl[..], as = funding, on = *axis == funding.0)]
        #[joins(left = root.perp_size[..], as = size, on = *axis == size.0)]
        #[joins(left = root.perp_funding[..], as = funding_index, on = *axis == funding_index.0)]
        #[joins(left = root.positions[..], as = position, on = *axis == position.0)]
        struct AccountPnlRow {
            #[from_axis(*axis)]
            account_id: AccountId,
            #[select(select = realized.1)]
            realized_delta: Option<Balance>,
            #[select(select = funding.1)]
            funding_delta: Option<Balance>,
            #[select(select = size.1)]
            position_size_delta: Option<NonZeroSidedSize>,
            #[select(select = funding_index.1)]
            funding_index_delta: Option<FundingIndexMantissa>,
            #[select(select = position.1.base)]
            position_size: Option<NonZeroSidedSize>,
            #[select(select = position.1.funding_index)]
            funding_index: Option<FundingIndexMantissa>,
            #[select(select = position.1.quote)]
            quote: Option<PositiveQuoteSize>,
        }
    }

    let rows = Root {
        accounts: vec![1, 2, 3],
        price_size_pnl: vec![(1, 100), (3, -30)],
        funding_index_pnl: vec![(1, 7), (2, 11)],
        perp_size: vec![(2, 2000), (3, -3000)],
        perp_funding: vec![(1, 70), (3, 90)],
        positions: vec![
            (
                1,
                Position {
                    base: 10,
                    funding_index: 700,
                    quote: 1000,
                },
            ),
            (
                3,
                Position {
                    base: -30,
                    funding_index: 900,
                    quote: 3000,
                },
            ),
        ],
    }
    .to_rows()
    .account_pnls;

    assert_eq!(rows.len(), 3);

    assert_eq!(rows[0].account_id, 1);
    assert_eq!(rows[0].realized_delta, Some(100));
    assert_eq!(rows[0].funding_delta, Some(7));
    assert_eq!(rows[0].position_size_delta, None);
    assert_eq!(rows[0].funding_index_delta, Some(70));
    assert_eq!(rows[0].position_size, Some(10));
    assert_eq!(rows[0].funding_index, Some(700));
    assert_eq!(rows[0].quote, Some(1000));

    assert_eq!(rows[1].account_id, 2);
    assert_eq!(rows[1].realized_delta, None);
    assert_eq!(rows[1].funding_delta, Some(11));
    assert_eq!(rows[1].position_size_delta, Some(2000));
    assert_eq!(rows[1].funding_index_delta, None);
    assert_eq!(rows[1].position_size, None);
    assert_eq!(rows[1].funding_index, None);
    assert_eq!(rows[1].quote, None);

    assert_eq!(rows[2].account_id, 3);
    assert_eq!(rows[2].realized_delta, Some(-30));
    assert_eq!(rows[2].funding_delta, None);
    assert_eq!(rows[2].position_size_delta, Some(-3000));
    assert_eq!(rows[2].funding_index_delta, Some(90));
    assert_eq!(rows[2].position_size, Some(-30));
    assert_eq!(rows[2].funding_index, Some(900));
    assert_eq!(rows[2].quote, Some(3000));
}
