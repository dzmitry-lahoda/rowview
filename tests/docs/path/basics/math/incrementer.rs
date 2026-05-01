//! Increments and tests fields from a constant or starting from a root value.

#[test]
fn increment_by_1() {
    struct Root {
        value: u32,
        axis: Vec<(char, f32)>,
    }

    #[rowview::rows(root = Root)]
    mod schema {

        #[rowset(name = incremented, axis = root.axis)]
        struct IncrementedRow {
            #[copy(increment = root.value + 1)]
            counter: u32,
            #[from_axis(axis.1)]
            rest: f32,
        }
    }

    let root = Root {
        value: 41,
        axis: vec![('a', 0.5), ('b', 111.111), ('c', 666.)],
    };
    let incremented_rows = root.to_rows().incremented;

    assert_eq!(incremented_rows.len(), 3);
    assert_eq!(incremented_rows.counter[0], 42);
    assert_eq!(incremented_rows.counter[1], 43);
    assert_eq!(incremented_rows.counter[2], 44);
    assert_eq!(incremented_rows.rest[0], 0.5);
    assert_eq!(incremented_rows.rest[1], 111.111);
    assert_eq!(incremented_rows.rest[2], 666.0);
}

#[test]
fn increment_from_plus_2() {
    struct Root {
        value: u32,
        axis: Vec<(char, f32)>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = incremented, axis = root.axis)]
        struct IncrementedRow {
            #[copy(increment = root.value + 2)]
            counter: u32,
            #[from_axis(axis.1)]
            rest: f32,
        }
    }

    let root = Root {
        value: 41,
        axis: vec![('a', 0.5), ('b', 111.111), ('c', 666.0)],
    };
    let incremented_rows = root.to_rows().incremented;

    assert_eq!(incremented_rows.len(), 3);
    assert_eq!(incremented_rows.counter[0], 43);
    assert_eq!(incremented_rows.counter[1], 44);
    assert_eq!(incremented_rows.counter[2], 45);
    assert_eq!(incremented_rows.rest[0], 0.5);
    assert_eq!(incremented_rows.rest[1], 111.111);
    assert_eq!(incremented_rows.rest[2], 666.0);
}
