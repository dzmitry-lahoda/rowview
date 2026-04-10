            // DSL variants to consider:
            // #[left_join(from = root.values, on = axis.0 == join.0, select = join.1)]
            // #[left_join(root.values, on = axis.0 == value.0, select = value.1)]
            // #[left_join(each value in root.values, on = axis.0 == value.0, select = value.1)]
            // #[join(left, from = root.values, on = axis.0 == item.0, select = item.1)]
            // #[join(kind = left, from = root.values, on = axis.0 == item.0, select = item.1)]
            //
            // Join modes to support:
            // #[join(kind = option, from = root.values, on = axis.0 == item.0, select = item.1)]
            // #[join(kind = must_panic, from = root.values, on = axis.0 == item.0, select = item.1)]
            // #[join(kind = must_result, from = root.values, on = axis.0 == item.0, select = item.1)]
            //
            // Positional joins are not axis joins; they assume same order as the axis.
            // #[zip_panic(from = root.values, select = item.1)]
            // #[zip_result(from = root.values, select = item.1)]
            // zip_panic/zip_result must fail when lengths differ.
            //
            // Multi-hop joins need dependency semantics:
            // Given axes x, y, and a:
            // - if x and y exist, y must exist for x
            // - if y exists, a must exist for y
            // - otherwise the whole chain is optional
            // #[join_chain(option, x -> y must -> a must, select = a.value)]