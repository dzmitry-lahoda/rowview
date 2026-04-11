//! If any axis exists, try to join with another axis.
// Multi-hop joins need dependency semantics:
// Given axes x, y, and a:
// - if x and y exist, y must exist for x
// - if y exists, a must exist for y
// - otherwise the whole chain is optional
// #[join_chain(option, x -> y must -> a must, select = a.value)]
