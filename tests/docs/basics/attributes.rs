//! Can set attributes on rows.

#[test]
fn rows_can_have_serde_attributes() {
    fn assert_serde<T>()
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de>,
    {
    }

    struct Root {
        axis: Vec<(u32, bool)>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[derive(serde::Serialize, serde::Deserialize)]
        #[rowset(name = numbers, axis = root.axis)]
        struct NumberRow {
            #[from_axis(axis.0)]
            id: u32,
        }

        #[derive(serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[rowset(name = flags, axis = root.axis)]
        struct FlagRow {
            #[from_axis(axis.0)]
            #[serde(rename = "flagId")]
            flag_id: u32,
            #[from_axis(axis.1)]
            flag_value: bool,
        }
    }

    assert_serde::<schema::NumberRow>();
    assert_serde::<schema::FlagRow>();

    let rows = Root {
        axis: vec![(1, true), (2, false)],
    }
    .to_rows();

    assert_eq!(rows.numbers[0].id, 1);
    assert_eq!(rows.flags[0].flag_id, 1);
    assert!(rows.flags[0].flag_value);
}
