use std::{rc::Rc, sync::Arc};

use crate::{
    model::{
        name::{ColumnName, PumpName, StreamName},
        option::options_builder::OptionsBuilder,
        query_plan::query_plan_node::{operation::LeafOperation, QueryPlanNodeLeaf},
    },
    stream_engine::{
        autonomous_executor::Timestamp,
        dependency_injection::{test_di::TestDI, DependencyInjection},
        pipeline::stream_model::StreamModel,
    },
    stream_engine::{
        autonomous_executor::{
            data::{
                column::stream_column::StreamColumns,
                column_values::ColumnValues,
                foreign_row::format::json::JsonObject,
                row::Row,
                value::sql_value::{nn_sql_value::NnSqlValue, SqlValue},
            },
            server::source::{
                net::{NetSourceServerActive, NetSourceServerStandby},
                SourceServerStandby,
            },
            test_support::foreign::source::TestSource,
        },
        pipeline::stream_model::stream_shape::StreamShape,
        RowRepository,
    },
};

impl NetSourceServerActive {
    pub(in crate::stream_engine) fn factory_with_test_source(inputs: Vec<JsonObject>) -> Self {
        let source = TestSource::start(inputs).unwrap();

        let options = OptionsBuilder::default()
            .add("PROTOCOL", "TCP")
            .add("REMOTE_HOST", source.host_ip().to_string())
            .add("REMOTE_PORT", source.port().to_string())
            .build();

        let server = NetSourceServerStandby::new(&options).unwrap();
        server.start().unwrap()
    }
}

impl StreamColumns {
    pub(in crate::stream_engine) fn factory_city_temperature(
        timestamp: Timestamp,
        city: &str,
        temperature: i32,
    ) -> Self {
        let mut column_values = ColumnValues::default();
        column_values
            .insert(
                ColumnName::new("timestamp".to_string()),
                SqlValue::NotNull(NnSqlValue::Timestamp(timestamp)),
            )
            .unwrap();
        column_values
            .insert(
                ColumnName::new("city".to_string()),
                SqlValue::NotNull(NnSqlValue::Text(city.to_string())),
            )
            .unwrap();
        column_values
            .insert(
                ColumnName::new("temperature".to_string()),
                SqlValue::NotNull(NnSqlValue::Integer(temperature)),
            )
            .unwrap();

        Self::new(Arc::new(StreamShape::fx_city_temperature()), column_values).unwrap()
    }

    pub(in crate::stream_engine) fn factory_trade(
        timestamp: Timestamp,
        ticker: &str,
        amount: i16,
    ) -> Self {
        let mut column_values = ColumnValues::default();
        column_values
            .insert(
                ColumnName::new("timestamp".to_string()),
                SqlValue::NotNull(NnSqlValue::Timestamp(timestamp)),
            )
            .unwrap();
        column_values
            .insert(
                ColumnName::new("ticker".to_string()),
                SqlValue::NotNull(NnSqlValue::Text(ticker.to_string())),
            )
            .unwrap();
        column_values
            .insert(
                ColumnName::new("amount".to_string()),
                SqlValue::NotNull(NnSqlValue::SmallInt(amount)),
            )
            .unwrap();

        Self::new(Arc::new(StreamShape::fx_trade()), column_values).unwrap()
    }

    pub(in crate::stream_engine) fn factory_no_promoted_rowtime(amount: i32) -> Self {
        let mut column_values = ColumnValues::default();
        column_values
            .insert(
                ColumnName::new("amount".to_string()),
                SqlValue::NotNull(NnSqlValue::Integer(amount)),
            )
            .unwrap();

        Self::new(
            Arc::new(StreamShape::fx_no_promoted_rowtime()),
            column_values,
        )
        .unwrap()
    }
}

impl Row {
    pub(in crate::stream_engine) fn factory_city_temperature(
        timestamp: Timestamp,
        city: &str,
        temperature: i32,
    ) -> Self {
        Self::new::<TestDI>(StreamColumns::factory_city_temperature(
            timestamp,
            city,
            temperature,
        ))
    }
    pub(in crate::stream_engine) fn factory_trade(
        timestamp: Timestamp,
        ticker: &str,
        amount: i16,
    ) -> Self {
        Self::new::<TestDI>(StreamColumns::factory_trade(timestamp, ticker, amount))
    }
}

impl QueryPlanNodeLeaf {
    pub(in crate::stream_engine) fn factory_with_pump_in<DI>(
        pump_name: PumpName,
        input: Vec<Row>,
        row_repo: &DI::RowRepositoryType,
    ) -> Self
    where
        DI: DependencyInjection,
    {
        let downstream_pumps = vec![pump_name.clone()];

        for row in input {
            row_repo.emit_owned(row, &downstream_pumps).unwrap();
        }

        Self {
            op: LeafOperation::Collect { pump: pump_name },
        }
    }
}

impl StreamName {
    pub(in crate::stream_engine) fn factory(name: &str) -> Self {
        Self::new(name.to_string())
    }
}

impl PumpName {
    pub(in crate::stream_engine) fn factory(name: &str) -> Self {
        Self::new(name.to_string())
    }
}