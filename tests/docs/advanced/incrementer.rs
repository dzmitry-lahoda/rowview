//! By closure or method

#[test]
fn increment_from_local_trait_method() {
    trait Next {
        fn next(self) -> Self;
    }

    impl Next for u32 {
        fn next(self) -> Self {
            self + 1
        }
    }

    struct Root {
        value: u32,
        axis: Vec<(char, f32)>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = incremented, axis = root.axis)]
        struct IncrementedRow {
            #[copy(increment = root.value.next())]
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
    assert_eq!(incremented_rows[0].counter, 42);
    assert_eq!(incremented_rows[1].counter, 43);
    assert_eq!(incremented_rows[2].counter, 44);
    assert_eq!(incremented_rows[0].rest, 0.5);
    assert_eq!(incremented_rows[1].rest, 111.111);
    assert_eq!(incremented_rows[2].rest, 666.0);
}
