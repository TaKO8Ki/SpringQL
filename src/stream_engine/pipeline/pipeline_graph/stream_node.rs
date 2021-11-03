use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{
    model::name::StreamName,
    stream_engine::pipeline::{
        foreign_stream_model::ForeignStreamModel, stream_model::StreamModel,
    },
};

#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub(in crate::stream_engine) enum StreamNode {
    Native(Arc<StreamModel>),
    Foreign(Arc<ForeignStreamModel>),
    VirtualRoot,
    VirtualLeaf { parent_foreign_stream: StreamName },
}

impl StreamNode {
    pub(in crate::stream_engine) fn name(&self) -> StreamName {
        match self {
            StreamNode::Native(stream) => stream.name().clone(),
            StreamNode::Foreign(stream) => stream.name().clone(),
            StreamNode::VirtualRoot => StreamName::virtual_root(),
            StreamNode::VirtualLeaf {
                parent_foreign_stream,
            } => StreamName::virtual_leaf(parent_foreign_stream.clone()),
        }
    }
}
