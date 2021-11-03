//! A PipelineGraph has a "virtual root stream", who has outgoing edges to all source foreign streams.
//! It also has "virtual leaf streams", who has an incoming edge from each sink foreign stream.

pub(in crate::stream_engine) mod edge;
pub(in crate::stream_engine) mod stream_node;

use std::{collections::HashMap, sync::Arc};

use petgraph::{
    graph::{DiGraph, EdgeReference, NodeIndex},
    visit::{EdgeRef, IntoEdgeReferences, IntoEdgesDirected},
};
use serde::{Deserialize, Serialize};

use self::{edge::Edge, stream_node::StreamNode};

use super::{
    foreign_stream_model::ForeignStreamModel,
    pump_model::PumpModel,
    server_model::{server_state::ServerState, ServerModel},
    stream_model::StreamModel,
};
use crate::{
    error::{Result, SpringError},
    model::name::{PumpName, StreamName},
    stream_engine::pipeline::{
        pump_model::pump_state::PumpState, server_model::server_type::ServerType,
    },
};
use anyhow::anyhow;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(in crate::stream_engine) struct PipelineGraph {
    graph: DiGraph<StreamNode, Edge>,
    stream_nodes: HashMap<StreamName, NodeIndex>,
}

impl Default for PipelineGraph {
    fn default() -> Self {
        let mut graph = DiGraph::new();
        let virtual_root_node = graph.add_node(StreamNode::VirtualRoot);

        let mut stream_nodes = HashMap::new();
        stream_nodes.insert(StreamName::virtual_root(), virtual_root_node);

        Self {
            graph,
            stream_nodes,
        }
    }
}

impl PipelineGraph {
    pub(super) fn add_stream(&mut self, stream: Arc<StreamModel>) -> Result<()> {
        let st_name = stream.name().clone();
        let st_node = self.graph.add_node(StreamNode::Native(stream));
        let _ = self.stream_nodes.insert(st_name, st_node);
        Ok(())
    }

    pub(super) fn add_foreign_stream(
        &mut self,
        foreign_stream: Arc<ForeignStreamModel>,
    ) -> Result<()> {
        let fst_name = foreign_stream.name().clone();
        let fst_node = self.graph.add_node(StreamNode::Foreign(foreign_stream));
        let _ = self.stream_nodes.insert(fst_name, fst_node);
        Ok(())
    }

    pub(super) fn get_pump(&self, name: &PumpName) -> Result<&PumpModel> {
        let edge = self._find_pump(name)?;
        if let Edge::Pump(pump) = edge.weight() {
            Ok(pump)
        } else {
            unreachable!()
        }
    }

    pub(super) fn add_pump(&mut self, pump: PumpModel) -> Result<()> {
        let upstream_node = self.stream_nodes.get(pump.upstream()).ok_or_else(|| {
            SpringError::Sql(anyhow!(
                r#"upstream "{}" does not exist in pipeline"#,
                pump.upstream()
            ))
        })?;
        let downstream_node = self.stream_nodes.get(pump.downstream()).ok_or_else(|| {
            SpringError::Sql(anyhow!(
                r#"downstream "{}" does not exist in pipeline"#,
                pump.downstream()
            ))
        })?;

        let _ = self
            .graph
            .add_edge(*upstream_node, *downstream_node, Edge::Pump(pump));

        Ok(())
    }

    pub(super) fn remove_pump(&mut self, name: &PumpName) -> Result<()> {
        let edge_idx = {
            let edge = self._find_pump(name)?;
            edge.id()
        };
        self.graph.remove_edge(edge_idx);
        Ok(())
    }

    pub(super) fn add_server(&mut self, server: ServerModel) -> Result<()> {
        let serving_to = server.serving_foreign_stream();

        match server.server_type() {
            ServerType::SourceNet => {
                let upstream_node = self
                    .stream_nodes
                    .get(&StreamName::virtual_root())
                    .expect("virtual root always available");
                let downstream_node =
                    self.stream_nodes.get(serving_to.name()).ok_or_else(|| {
                        SpringError::Sql(anyhow!(
                            r#"downstream "{}" does not exist in pipeline"#,
                            serving_to.name()
                        ))
                    })?;
                let _ = self
                    .graph
                    .add_edge(*upstream_node, *downstream_node, Edge::Source(server));
            }
            ServerType::SinkNet => {
                let upstream_node = self.stream_nodes.get(serving_to.name()).ok_or_else(|| {
                    SpringError::Sql(anyhow!(
                        r#"upstream "{}" does not exist in pipeline"#,
                        serving_to.name()
                    ))
                })?;
                let downstream_node = self.graph.add_node(StreamNode::VirtualLeaf {
                    parent_foreign_stream: serving_to.name().clone(),
                });
                let _ = self
                    .graph
                    .add_edge(*upstream_node, downstream_node, Edge::Sink(server));
            }
        }
        Ok(())
    }

    pub(in crate::stream_engine) fn source_server_state(
        &self,
        serving_foreign_stream: &StreamName,
    ) -> ServerState {
        let fst_node = self
            .graph
            .node_indices()
            .find(|n| &self.graph[*n].name() == serving_foreign_stream)
            .unwrap();

        if self._at_least_one_started_path_to_sink(fst_node) {
            ServerState::Started
        } else {
            ServerState::Stopped
        }
    }

    pub(in crate::stream_engine) fn as_petgraph(&self) -> &DiGraph<StreamNode, Edge> {
        &self.graph
    }

    fn _find_pump(&self, name: &PumpName) -> Result<EdgeReference<Edge>> {
        self.graph
            .edge_references()
            .find_map(|edge| {
                if let Edge::Pump(pump) = edge.weight() {
                    (pump.name() == name).then(|| edge)
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                SpringError::Sql(anyhow!(r#"pump "{}" does not exist in pipeline"#, name))
            })
    }

    fn _at_least_one_started_path_to_sink(&self, node: NodeIndex) -> bool {
        let mut outgoing_edges = self
            .graph
            .edges_directed(node, petgraph::Direction::Outgoing);

        if outgoing_edges
            .clone()
            .any(|edge| matches!(edge.weight(), Edge::Sink(_)))
        {
            true
        } else if outgoing_edges.clone().count() == 0 {
            false
        } else {
            outgoing_edges.any(|started_edge| {
                let next_node = started_edge.target();
                self._at_least_one_started_path_to_sink(next_node)
            })
        }
    }
}
