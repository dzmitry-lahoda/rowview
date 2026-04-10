// #![forbid(clippy::clone_on_copy, clippy::redundant_clone)]
// impl Clone for Clonable {
//     fn clone(&self) -> Self {
//         panic!("Clonable should not be cloned");
//     }
// }

// struct Clonable {
//     bytes: Vec<u8>,
// }

// struct Root {
//     axis: Vec<(u32, Clonable)>,
// }

// #[rowview::rows(root = Root)]
// mod schema {
//     #[rowset(name = axis_rows, axis = root.axis)]
//     struct AxisRow {
//         #[from_axis(axis.1[0])]
//         axis_value: u8,
//     }
// }

// #[test]
// fn generated_rows_do_not_require_clone_on_input_data() {
//     let root = Root {
//         meta: (7, NotClone { _bytes: vec![1, 2, 3] }),
//         axis: vec![
//             (10, NotClone { _bytes: vec![4] }),
//             (11, NotClone { _bytes: vec![5, 6] }),
//         ],
//     };

//     let rows = root.to_rows();

//     assert_eq!(rows.root_rows.len(), 1);
//     assert_eq!(rows.root_rows[0].root_id, 7);

//     assert_eq!(rows.axis_rows.len(), 2);
//     assert_eq!(rows.axis_rows[0].root_id, 7);
//     assert_eq!(rows.axis_rows[0].axis_id, 10);
//     assert_eq!(rows.axis_rows[1].root_id, 7);
//     assert_eq!(rows.axis_rows[1].axis_id, 11);
// }
