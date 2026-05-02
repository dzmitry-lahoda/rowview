use std::marker::PhantomData;

pub mod axis {
    #[allow(non_camel_case_types)]
    #[derive(Clone, Copy, Debug)]
    pub struct _0;

    #[allow(non_camel_case_types)]
    #[derive(Clone, Copy, Debug)]
    pub struct _1;
}

pub mod vals {
    #[allow(non_camel_case_types)]
    #[derive(Clone, Copy, Debug)]
    pub struct _0;

    #[allow(non_camel_case_types)]
    #[derive(Clone, Copy, Debug)]
    pub struct _1;
}

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

pub struct SelectJoinLeft<Row, Axis, Join, Predicate> {
    axis: Axis,
    join: Join,
    predicate: Predicate,
    _types: PhantomData<fn() -> (Row, Axis, Join, Predicate)>,
}

pub struct SelectProject<Row, Axis, Join, Predicate, Projection, Projected> {
    axis: Axis,
    join: Join,
    predicate: Predicate,
    projection: Projection,
    _types: PhantomData<fn() -> (Row, Axis, Join, Predicate, Projection, Projected)>,
}

pub struct SelectMap<Row, Axis, Projection, Projected> {
    axis: Axis,
    projection: Projection,
    _types: PhantomData<fn() -> (Row, Axis, Projection, Projected)>,
}

pub struct SelectLeftProject<
    Row,
    Axis,
    Join,
    Predicate,
    MatchedProjection,
    MatchedProjected,
    MissingProjection,
    MissingProjected,
> {
    axis: Axis,
    join: Join,
    predicate: Predicate,
    matched_projection: MatchedProjection,
    missing_projection: MissingProjection,
    _types: PhantomData<
        fn() -> (
            Row,
            Axis,
            Join,
            Predicate,
            MatchedProjection,
            MatchedProjected,
            MissingProjection,
            MissingProjected,
        ),
    >,
}

pub trait QuerySource {
    type Item;

    fn iter(&self) -> Box<dyn Iterator<Item = &Self::Item> + '_>;
}

pub trait TupleField<const INDEX: usize> {
    type Output;

    fn field(&self) -> Self::Output;
}

impl<T0, T1> TupleField<0> for (T0, T1)
where
    T0: Clone,
{
    type Output = T0;

    fn field(&self) -> Self::Output {
        self.0.clone()
    }
}

impl<T0, T1> TupleField<1> for (T0, T1)
where
    T1: Clone,
{
    type Output = T1;

    fn field(&self) -> Self::Output {
        self.1.clone()
    }
}

pub trait Expr<AxisItem, JoinItem> {
    type Output;

    fn eval(&self, axis_item: &AxisItem, join_item: &JoinItem) -> Self::Output;
}

impl<AxisItem, JoinItem> Expr<AxisItem, JoinItem> for axis::_0
where
    AxisItem: TupleField<0>,
{
    type Output = <AxisItem as TupleField<0>>::Output;

    fn eval(&self, axis_item: &AxisItem, _join_item: &JoinItem) -> Self::Output {
        axis_item.field()
    }
}

impl<AxisItem, JoinItem> Expr<AxisItem, JoinItem> for axis::_1
where
    AxisItem: TupleField<1>,
{
    type Output = <AxisItem as TupleField<1>>::Output;

    fn eval(&self, axis_item: &AxisItem, _join_item: &JoinItem) -> Self::Output {
        axis_item.field()
    }
}

impl<AxisItem, JoinItem> Expr<AxisItem, JoinItem> for vals::_0
where
    JoinItem: TupleField<0>,
{
    type Output = <JoinItem as TupleField<0>>::Output;

    fn eval(&self, _axis_item: &AxisItem, join_item: &JoinItem) -> Self::Output {
        join_item.field()
    }
}

impl<AxisItem, JoinItem> Expr<AxisItem, JoinItem> for vals::_1
where
    JoinItem: TupleField<1>,
{
    type Output = <JoinItem as TupleField<1>>::Output;

    fn eval(&self, _axis_item: &AxisItem, join_item: &JoinItem) -> Self::Output {
        join_item.field()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EqExpr<Left, Right> {
    left: Left,
    right: Right,
}

pub trait ExprExt: Sized {
    fn eq<Right>(self, right: Right) -> EqExpr<Self, Right> {
        EqExpr { left: self, right }
    }

    fn some(self) -> SomeExpr<Self> {
        SomeExpr { expr: self }
    }
}

impl<T> ExprExt for T {}

impl<AxisItem, JoinItem, Left, Right> Expr<AxisItem, JoinItem> for EqExpr<Left, Right>
where
    Left: Expr<AxisItem, JoinItem>,
    Right: Expr<AxisItem, JoinItem>,
    Left::Output: PartialEq<Right::Output>,
{
    type Output = bool;

    fn eval(&self, axis_item: &AxisItem, join_item: &JoinItem) -> Self::Output {
        self.left.eval(axis_item, join_item) == self.right.eval(axis_item, join_item)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct On<Predicate> {
    predicate: Predicate,
}

pub const  fn on<Predicate>(predicate: Predicate) -> On<Predicate> {
    On { predicate }
}

#[derive(Clone, Copy, Debug)]
pub struct SomeExpr<Inner> {
    expr: Inner,
}

impl<AxisItem, JoinItem, Inner> Expr<AxisItem, JoinItem> for SomeExpr<Inner>
where
    Inner: Expr<AxisItem, JoinItem>,
{
    type Output = Option<Inner::Output>;

    fn eval(&self, axis_item: &AxisItem, join_item: &JoinItem) -> Self::Output {
        Some(self.expr.eval(axis_item, join_item))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NoneExpr<T> {
    _types: PhantomData<fn() -> T>,
}

pub const  fn none<T>() -> NoneExpr<T> {
    NoneExpr {
        _types: PhantomData,
    }
}

impl<AxisItem, JoinItem, T> Expr<AxisItem, JoinItem> for NoneExpr<T> {
    type Output = Option<T>;

    fn eval(&self, _axis_item: &AxisItem, _join_item: &JoinItem) -> Self::Output {
        None
    }
}

pub trait SelectPredicate<AxisItem, JoinItem> {
    fn test(&mut self, axis_item: &AxisItem, join_item: &JoinItem) -> bool;
}

impl<AxisItem, JoinItem, Predicate> SelectPredicate<AxisItem, JoinItem> for Predicate
where
    Predicate: FnMut(&AxisItem, &JoinItem) -> bool,
{
    fn test(&mut self, axis_item: &AxisItem, join_item: &JoinItem) -> bool {
        self(axis_item, join_item)
    }
}

impl<AxisItem, JoinItem, Predicate> SelectPredicate<AxisItem, JoinItem> for On<Predicate>
where
    Predicate: Expr<AxisItem, JoinItem, Output = bool>,
{
    fn test(&mut self, axis_item: &AxisItem, join_item: &JoinItem) -> bool {
        self.predicate.eval(axis_item, join_item)
    }
}

pub trait ProjectExpr<AxisItem, JoinItem> {
    type Output;

    fn project(&mut self, axis_item: &AxisItem, join_item: &JoinItem) -> Self::Output;
}

impl<AxisItem, JoinItem, Projection, Projected> ProjectExpr<AxisItem, JoinItem> for Projection
where
    Projection: FnMut(&AxisItem, &JoinItem) -> Projected,
{
    type Output = Projected;

    fn project(&mut self, axis_item: &AxisItem, join_item: &JoinItem) -> Self::Output {
        self(axis_item, join_item)
    }
}

pub trait SelectAxisProject<AxisItem> {
    type Output;

    fn project_axis(&mut self, axis_item: &AxisItem) -> Self::Output;
}

impl<AxisItem, Projection, Projected> SelectAxisProject<AxisItem> for Projection
where
    Projection: FnMut(&AxisItem) -> Projected,
{
    type Output = Projected;

    fn project_axis(&mut self, axis_item: &AxisItem) -> Self::Output {
        self(axis_item)
    }
}

impl<AxisItem, JoinItem, E0, E1> ProjectExpr<AxisItem, JoinItem> for (E0, E1)
where
    E0: Expr<AxisItem, JoinItem>,
    E1: Expr<AxisItem, JoinItem>,
{
    type Output = (E0::Output, E1::Output);

    fn project(&mut self, axis_item: &AxisItem, join_item: &JoinItem) -> Self::Output {
        (
            self.0.eval(axis_item, join_item),
            self.1.eval(axis_item, join_item),
        )
    }
}

impl<AxisItem, JoinItem, E0, E1, E2, E3> ProjectExpr<AxisItem, JoinItem> for (E0, E1, E2, E3)
where
    E0: Expr<AxisItem, JoinItem>,
    E1: Expr<AxisItem, JoinItem>,
    E2: Expr<AxisItem, JoinItem>,
    E3: Expr<AxisItem, JoinItem>,
{
    type Output = (E0::Output, E1::Output, E2::Output, E3::Output);

    fn project(&mut self, axis_item: &AxisItem, join_item: &JoinItem) -> Self::Output {
        (
            self.0.eval(axis_item, join_item),
            self.1.eval(axis_item, join_item),
            self.2.eval(axis_item, join_item),
            self.3.eval(axis_item, join_item),
        )
    }
}

impl<AxisItem, E0, E1> SelectAxisProject<AxisItem> for (E0, E1)
where
    E0: Expr<AxisItem, ()>,
    E1: Expr<AxisItem, ()>,
{
    type Output = (E0::Output, E1::Output);

    fn project_axis(&mut self, axis_item: &AxisItem) -> Self::Output {
        (self.0.eval(axis_item, &()), self.1.eval(axis_item, &()))
    }
}

impl<AxisItem, E0, E1, E2, E3> SelectAxisProject<AxisItem> for (E0, E1, E2, E3)
where
    E0: Expr<AxisItem, ()>,
    E1: Expr<AxisItem, ()>,
    E2: Expr<AxisItem, ()>,
    E3: Expr<AxisItem, ()>,
{
    type Output = (E0::Output, E1::Output, E2::Output, E3::Output);

    fn project_axis(&mut self, axis_item: &AxisItem) -> Self::Output {
        (
            self.0.eval(axis_item, &()),
            self.1.eval(axis_item, &()),
            self.2.eval(axis_item, &()),
            self.3.eval(axis_item, &()),
        )
    }
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
    pub const  fn from<Axis>(axis: Axis) -> SelectFrom<Row, Axis>
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
    pub const fn project<Projection, Projected>(
        self,
        projection: Projection,
    ) -> SelectMap<Row, Axis, Projection, Projected>
    where
        Projection: SelectAxisProject<Axis::Item, Output = Projected>,
        Projected: Into<Row>,
    {
        SelectMap {
            axis: self.axis,
            projection,
            _types: PhantomData,
        }
    }

    pub const fn join_must<Join, Predicate>(
        self,
        join: Join,
        predicate: Predicate,
    ) -> SelectJoinMust<Row, Axis, Join, Predicate>
    where
        Join: QuerySource,
        Predicate: SelectPredicate<Axis::Item, Join::Item>,
    {
        SelectJoinMust {
            axis: self.axis,
            join,
            predicate,
            _types: PhantomData,
        }
    }

    pub const  fn join_left<Join, Predicate>(
        self,
        join: Join,
        predicate: Predicate,
    ) -> SelectJoinLeft<Row, Axis, Join, Predicate>
    where
        Join: QuerySource,
        Predicate: SelectPredicate<Axis::Item, Join::Item>,
    {
        SelectJoinLeft {
            axis: self.axis,
            join,
            predicate,
            _types: PhantomData,
        }
    }
}

impl<Row, Axis, Projection, Projected> SelectMap<Row, Axis, Projection, Projected>
where
    Axis: QuerySource,
    Projection: SelectAxisProject<Axis::Item, Output = Projected>,
    Projected: Into<Row>,
    Row: layout::SOA,
    <Row as layout::SOA>::Type: FromIterator<Row>,
{
    pub fn execute(self) -> <Row as layout::SOA>::Type {
        let mut projection = self.projection;

        self.axis
            .iter()
            .map(|axis_item| projection.project_axis(axis_item).into())
            .collect()
    }
}

impl<Row, Axis, Join, Predicate> SelectJoinMust<Row, Axis, Join, Predicate>
where
    Axis: QuerySource,
    Join: QuerySource,
    Predicate: SelectPredicate<Axis::Item, Join::Item>,
{
    pub const  fn project<Projection, Projected>(
        self,
        projection: Projection,
    ) -> SelectProject<Row, Axis, Join, Predicate, Projection, Projected>
    where
        Projection: ProjectExpr<Axis::Item, Join::Item, Output = Projected>,
        Projected: Into<Row>,
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

impl<Row, Axis, Join, Predicate> SelectJoinLeft<Row, Axis, Join, Predicate>
where
    Axis: QuerySource,
    Join: QuerySource,
    Predicate: SelectPredicate<Axis::Item, Join::Item>,
{
    pub const  fn project<MatchedProjection, MatchedProjected, MissingProjection, MissingProjected>(
        self,
        matched_projection: MatchedProjection,
        missing_projection: MissingProjection,
    ) -> SelectLeftProject<
        Row,
        Axis,
        Join,
        Predicate,
        MatchedProjection,
        MatchedProjected,
        MissingProjection,
        MissingProjected,
    >
    where
        MatchedProjection: ProjectExpr<Axis::Item, Join::Item, Output = MatchedProjected>,
        MatchedProjected: Into<Row>,
        MissingProjection: SelectAxisProject<Axis::Item, Output = MissingProjected>,
        MissingProjected: Into<Row>,
    {
        SelectLeftProject {
            axis: self.axis,
            join: self.join,
            predicate: self.predicate,
            matched_projection,
            missing_projection,
            _types: PhantomData,
        }
    }
}

impl<Row, Axis, Join, Predicate, Projection, Projected>
    SelectProject<Row, Axis, Join, Predicate, Projection, Projected>
where
    Axis: QuerySource,
    Join: QuerySource,
    Predicate: SelectPredicate<Axis::Item, Join::Item>,
    Projection: ProjectExpr<Axis::Item, Join::Item, Output = Projected>,
    Projected: Into<Row>,
    Row: layout::SOA,
    <Row as layout::SOA>::Type: FromIterator<Row>,
{
    pub const  fn execute(self) -> <Row as layout::SOA>::Type {
        let mut predicate = self.predicate;
        let mut projection = self.projection;

        self.axis
            .iter()
            .map(|axis_item| {
                let mut matching_join = None;
                for join_item in self.join.iter() {
                    if predicate.test(axis_item, join_item) {
                        matching_join = Some(join_item);
                        break;
                    }
                }

                projection
                    .project(
                        axis_item,
                        matching_join.expect("rowview must join found no matching item"),
                    )
                    .into()
            })
            .collect()
    }
}

impl<
    Row,
    Axis,
    Join,
    Predicate,
    MatchedProjection,
    MatchedProjected,
    MissingProjection,
    MissingProjected,
>
    SelectLeftProject<
        Row,
        Axis,
        Join,
        Predicate,
        MatchedProjection,
        MatchedProjected,
        MissingProjection,
        MissingProjected,
    >
where
    Axis: QuerySource,
    Join: QuerySource,
    Predicate: SelectPredicate<Axis::Item, Join::Item>,
    MatchedProjection: ProjectExpr<Axis::Item, Join::Item, Output = MatchedProjected>,
    MatchedProjected: Into<Row>,
    MissingProjection: SelectAxisProject<Axis::Item, Output = MissingProjected>,
    MissingProjected: Into<Row>,
    Row: layout::SOA,
    <Row as layout::SOA>::Type: FromIterator<Row>,
{
    pub const  fn execute(self) -> <Row as layout::SOA>::Type {
        let mut predicate = self.predicate;
        let mut matched_projection = self.matched_projection;
        let mut missing_projection = self.missing_projection;

        self.axis
            .iter()
            .map(|axis_item| {
                let mut matching_join = None;
                for join_item in self.join.iter() {
                    if predicate.test(axis_item, join_item) {
                        matching_join = Some(join_item);
                        break;
                    }
                }

                if let Some(join_item) = matching_join {
                    matched_projection.project(axis_item, join_item).into()
                } else {
                    missing_projection.project_axis(axis_item).into()
                }
            })
            .collect()
    }
}
