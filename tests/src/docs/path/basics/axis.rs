use std::collections::{BTreeSet, HashSet};

#[test]
fn one() {
    struct Root {
        axis: Vec<(char, f32)>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = axis_one, axis = root.axis)]
        struct OneRow {
            #[from_axis(axis.0)]
            id: char,
            #[from_axis(axis.1)]
            rest: f32,
        }
    }

    let root = Root {
        axis: vec![('a', 0.5), ('b', 111.111), ('c', 666.)],
    };
    let incremented_rows = root.to_rows().axis_one;

    assert_eq!(incremented_rows.len(), 3);
    assert_eq!(incremented_rows.id[0], 'a');
    assert_eq!(incremented_rows.id[1], 'b');
    assert_eq!(incremented_rows.id[2], 'c');
    assert_eq!(incremented_rows.rest[0], 0.5);
    assert_eq!(incremented_rows.rest[1], 111.111);
    assert_eq!(incremented_rows.rest[2], 666.0);
}

#[test]
fn hash_set() {
    struct Root {
        axis: HashSet<(u32, u32)>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = axis_rows, axis = root.axis)]
        struct AxisRow {
            #[from_axis(axis.0)]
            id: u32,
            #[from_axis(axis.1)]
            value: u32,
        }
    }

    let root = Root {
        axis: HashSet::from([(1, 10), (2, 20), (3, 30)]),
    };
    let rows = root.to_rows().axis_rows;

    assert_eq!(rows.len(), 3);

    let mut pairs: Vec<_> = rows.iter().map(|row| (*row.id, *row.value)).collect();
    pairs.sort_unstable();

    assert_eq!(pairs, vec![(1, 10), (2, 20), (3, 30)]);
}

#[test]
fn btree_set() {
    struct Root {
        axis: BTreeSet<(u32, u32)>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = axis_rows, axis = root.axis)]
        struct AxisRow {
            #[from_axis(axis.0)]
            id: u32,
            #[from_axis(axis.1)]
            value: u32,
        }
    }

    let root = Root {
        axis: BTreeSet::from([(3, 30), (1, 10), (2, 20)]),
    };
    let rows = root.to_rows().axis_rows;

    assert_eq!(rows.len(), 3);
    assert_eq!(rows.id[0], 1);
    assert_eq!(rows.value[0], 10);
    assert_eq!(rows.id[1], 2);
    assert_eq!(rows.value[1], 20);
    assert_eq!(rows.id[2], 3);
    assert_eq!(rows.value[2], 30);
}

/// 3 row sets from 3 separate axes.
#[test]
fn three() {
    struct Root {
        letters: Vec<(char, f32)>,
        numbers: Vec<(u32, bool)>,
        words: Vec<(&'static str, i16)>,
    }

    #[rowview::rows(root = Root)]
    mod schema {
        #[rowset(name = letters, axis = root.letters)]
        struct LetterRow {
            #[from_axis(axis.0)]
            id: char,
            #[from_axis(axis.1)]
            rest: f32,
        }

        #[rowset(name = numbers, axis = root.numbers)]
        struct NumberRow {
            #[from_axis(axis.0)]
            id: u32,
            #[from_axis(axis.1)]
            active: bool,
        }

        #[rowset(name = words, axis = root.words)]
        struct WordRow {
            #[from_axis(axis.0)]
            word: &'static str,
            #[from_axis(axis.1)]
            score: i16,
        }
    }

    let root = Root {
        letters: vec![('a', 0.5), ('b', 1.5)],
        numbers: vec![(1, true), (2, false), (3, true)],
        words: vec![("alpha", 10), ("beta", 20)],
    };
    let rows = root.to_rows();

    assert_eq!(rows.letters.len(), 2);
    assert_eq!(rows.letters.id[0], 'a');
    assert_eq!(rows.letters.id[1], 'b');
    assert_eq!(rows.letters.rest[0], 0.5);
    assert_eq!(rows.letters.rest[1], 1.5);

    assert_eq!(rows.numbers.len(), 3);
    assert_eq!(rows.numbers.id[0], 1);
    assert_eq!(rows.numbers.id[1], 2);
    assert_eq!(rows.numbers.id[2], 3);
    assert!(rows.numbers.active[0]);
    assert!(!rows.numbers.active[1]);
    assert!(rows.numbers.active[2]);

    assert_eq!(rows.words.len(), 2);
    assert_eq!(rows.words.word[0], "alpha");
    assert_eq!(rows.words.word[1], "beta");
    assert_eq!(rows.words.score[0], 10);
    assert_eq!(rows.words.score[1], 20);
}

#[test]
fn axis_element_position_value() {
    struct EnumeratedRoot {
        axis: Vec<(char, f32)>,
    }

    #[rowview::rows(root = EnumeratedRoot)]
    mod schema {
        #[rowset(name = axis_one, axis = root.axis)]
        struct OneRow {
            #[from_index(axis)]
            index: usize,
            #[from_index(axis)]
            index_capped: u32,
            #[from_axis(axis.1)]
            rest: f32,
        }
    }

    let rows = EnumeratedRoot {
        axis: vec![('a', 0.5), ('b', 111.111), ('c', 666.)],
    }
    .to_rows();

    assert_eq!(rows.axis_one.len(), 3);
    assert_eq!(rows.axis_one.index[0], 0);
    assert_eq!(rows.axis_one.index[1], 1);
    assert_eq!(rows.axis_one.index[2], 2);
    assert_eq!(rows.axis_one.index_capped[0], 0);
    assert_eq!(rows.axis_one.index_capped[1], 1);
    assert_eq!(rows.axis_one.index_capped[2], 2);
}
