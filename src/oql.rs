use std::marker::PhantomData;

#[allow(non_camel_case_types)]
pub struct select<Row>(PhantomData<fn() -> Row>);

pub struct SelectFrom<Row, Axis> {
    axis: Axis,
    _types: PhantomData<fn() -> (Row, Axis)>,
}

pub struct SelectJoinMust<Row, Axis, Join, Predicate> {
    axis: Axis,
    join: Join,
    predicate: Predicate,
    _types: PhantomData<fn() -> (Row, Axis, Join, Predicate)>,
}

pub struct SelectProject<Row, Axis, Join, Predicate, Projection> {
    axis: Axis,
    join: Join,
    predicate: Predicate,
    projection: Projection,
    _types: PhantomData<fn() -> (Row, Axis, Join, Predicate, Projection)>,
}

pub trait QuerySource {
    type Item;

    fn iter(&self) -> Box<dyn Iterator<Item = &Self::Item> + '_>;
}

impl<T> QuerySource for &Vec<T> {
    type Item = T;

    fn iter(&self) -> Box<dyn Iterator<Item = &Self::Item> + '_> {
        Box::new(self.as_slice().iter())
    }
}

impl<T> QuerySource for &[T] {
    type Item = T;

    fn iter(&self) -> Box<dyn Iterator<Item = &Self::Item> + '_> {
        Box::new((*self).iter())
    }
}

impl<Row> select<Row> {
    pub fn from<Axis>(axis: Axis) -> SelectFrom<Row, Axis>
    where
        Axis: QuerySource,
    {
        SelectFrom {
            axis,
            _types: PhantomData,
        }
    }
}

impl<Row, Axis> SelectFrom<Row, Axis>
where
    Axis: QuerySource,
{
    pub fn join_must<Join, Predicate>(
        self,
        join: Join,
        predicate: Predicate,
    ) -> SelectJoinMust<Row, Axis, Join, Predicate>
    where
        Join: QuerySource,
        Predicate: FnMut(&Axis::Item, &Join::Item) -> bool,
    {
        SelectJoinMust {
            axis: self.axis,
            join,
            predicate,
            _types: PhantomData,
        }
    }
}

impl<Row, Axis, Join, Predicate> SelectJoinMust<Row, Axis, Join, Predicate>
where
    Axis: QuerySource,
    Join: QuerySource,
    Predicate: FnMut(&Axis::Item, &Join::Item) -> bool,
{
    pub fn project<Projection>(
        self,
        projection: Projection,
    ) -> SelectProject<Row, Axis, Join, Predicate, Projection>
    where
        Projection: FnMut(&Axis::Item, &Join::Item) -> Row,
    {
        SelectProject {
            axis: self.axis,
            join: self.join,
            predicate: self.predicate,
            projection,
            _types: PhantomData,
        }
    }
}

impl<Row, Axis, Join, Predicate, Projection> SelectProject<Row, Axis, Join, Predicate, Projection>
where
    Axis: QuerySource,
    Join: QuerySource,
    Predicate: FnMut(&Axis::Item, &Join::Item) -> bool,
    Projection: FnMut(&Axis::Item, &Join::Item) -> Row,
    Row: layout::SOA,
    <Row as layout::SOA>::Type: FromIterator<Row>,
{
    pub fn execute(self) -> <Row as layout::SOA>::Type {
        let mut predicate = self.predicate;
        let mut projection = self.projection;

        self.axis
            .iter()
            .map(|axis_item| {
                let mut matching_join = None;
                for join_item in self.join.iter() {
                    if predicate(axis_item, join_item) {
                        matching_join = Some(join_item);
                        break;
                    }
                }

                projection(
                    axis_item,
                    matching_join.expect("rowview must join found no matching item"),
                )
            })
            .collect()
    }
}
