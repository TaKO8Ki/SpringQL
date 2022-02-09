#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub(crate) struct ExprLabelGenerator {
    value: u16,
    aggr: u16,
}

impl ExprLabelGenerator {
    pub(crate) fn next_value(&mut self) -> ValueExprLabel {
        let label = ValueExprLabel(self.value);
        self.value += 1;
        label
    }

    pub(crate) fn next_aggr(&mut self) -> AggrExprLabel {
        let label = AggrExprLabel(self.aggr);
        self.aggr += 1;
        label
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct ValueExprLabel(u16);

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct AggrExprLabel(u16);
