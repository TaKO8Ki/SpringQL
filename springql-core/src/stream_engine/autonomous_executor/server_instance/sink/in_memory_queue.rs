use crate::error::Result;
use crate::pipeline::name::QueueName;
use crate::pipeline::option::in_memory_queue_server_options::InMemoryQueueServerOptions;
use crate::stream_engine::in_memory_queue_repository::InMemoryQueueRepository;
use crate::{
    pipeline::option::Options,
    stream_engine::autonomous_executor::row::foreign_row::foreign_sink_row::ForeignSinkRow,
};

use super::SinkServerInstance;

#[derive(Debug)]
pub(in crate::stream_engine) struct InMemoryQueueSinkServerInstance(QueueName);

impl SinkServerInstance for InMemoryQueueSinkServerInstance {
    fn start(options: &Options) -> Result<Self>
    where
        Self: Sized,
    {
        let options = InMemoryQueueServerOptions::try_from(options)?;
        let queue_name = options.queue_name;
        InMemoryQueueRepository::instance().create(queue_name.clone())?;
        Ok(Self(queue_name))
    }

    fn send_row(&mut self, row: ForeignSinkRow) -> Result<()> {
        let q = InMemoryQueueRepository::instance().get(&self.0)?;
        q.push(row);
        Ok(())
    }
}