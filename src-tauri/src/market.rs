use std::collections::{BTreeMap, HashMap};

use crate::model::{
    MarketBasketSummary, MarketCityRow, MarketCurrencyPulse, MarketEvidenceDataset,
    MarketMetricContext, MarketPriceLedgerRow, MarketResourceLedgerRow, MarketScalarLedgerRow,
    MarketTermsOfTradeSummary, MarketTradePoint, MarketWarehouseProjection, MarketWorkspace,
};

pub(crate) fn build_workspace(
    evidence: MarketEvidenceDataset,
    warehouse_history_available: bool,
) -> MarketWorkspace {
    let Some(projection) = evidence.projection.as_ref() else {
        return MarketWorkspace {
            analysis_context: evidence.analysis_context,
            available: false,
            partial: evidence.coverage_status.as_deref() == Some("partial"),
            coverage_status: evidence.coverage_status,
            history_records: evidence.history_records,
            row_count: evidence.row_count,
            city_scope_count: evidence.snapshot_scopes,
            warehouse_history_available,
            warnings: evidence.warnings,
            currencies: Vec::new(),
            trade_history: Vec::new(),
            resource_ledger: Vec::new(),
            price_ledger: Vec::new(),
            scalar_ledger: Vec::new(),
            cities: Vec::new(),
            baskets: evidence.baskets,
            scenarios: evidence.scenarios,
            metric_contexts: Vec::new(),
            terms_of_trade: Vec::new(),
            reserves_available: false,
            terms_of_trade_available: false,
            limitations: limitations(),
        };
    };

    let record_by_hash = projection
        .records
        .iter()
        .map(|record| (record.record_hash.as_str(), record))
        .collect::<HashMap<_, _>>();
    let latest_record_hash = projection
        .records
        .iter()
        .max_by_key(|record| record.ordinal)
        .map(|record| record.record_hash.as_str());
    let trade_history = trade_history(projection, &record_by_hash);
    let latest_trades = selected_head_facts(
        &projection.trades,
        latest_record_hash,
        |fact| fact.scope_kind.as_deref(),
        |fact| fact.record_hash.as_deref(),
    );
    let resource_ledger = resource_ledger(&latest_trades);
    let currencies = currency_pulses(projection, &resource_ledger);
    let price_ledger = price_ledger(projection, latest_record_hash);
    let scalar_ledger = scalar_ledger(projection, latest_record_hash);
    let cities = city_ledger(projection);
    let mut baskets = builtin_baskets(projection, latest_record_hash);
    baskets.extend(evidence.baskets.into_iter().map(|mut basket| {
        evaluate_basket(&mut basket, projection, latest_record_hash);
        basket
    }));
    let metric_contexts = metric_contexts(projection);
    let terms_of_trade = terms_of_trade(&baskets, projection);
    let scenarios = evaluate_scenarios(evidence.scenarios, &currencies, &scalar_ledger);

    MarketWorkspace {
        analysis_context: evidence.analysis_context,
        available: !projection.records.is_empty()
            || !projection.prices.is_empty()
            || !projection.trades.is_empty()
            || !projection.scalars.is_empty(),
        partial: evidence.coverage_status.as_deref() == Some("partial"),
        coverage_status: evidence.coverage_status,
        history_records: evidence.history_records,
        row_count: evidence.row_count,
        city_scope_count: cities.len().min(u32::MAX as usize) as u32,
        warehouse_history_available,
        warnings: evidence.warnings,
        currencies,
        trade_history,
        resource_ledger,
        price_ledger,
        scalar_ledger,
        cities,
        baskets,
        scenarios,
        metric_contexts,
        terms_of_trade: terms_of_trade.clone(),
        reserves_available: false,
        terms_of_trade_available: !terms_of_trade.is_empty(),
        limitations: limitations(),
    }
}

fn trade_history(
    projection: &MarketWarehouseProjection,
    record_by_hash: &HashMap<&str, &crate::model::MarketWarehouseRecord>,
) -> Vec<MarketTradePoint> {
    if !projection.analytical_trade_history.is_empty() {
        return projection.analytical_trade_history.clone();
    }
    let mut totals = BTreeMap::<(String, String, String), (f64, f64)>::new();
    for fact in projection
        .trades
        .iter()
        .filter(|fact| fact.scope_kind.is_none())
    {
        let Some(record_hash) = fact.record_hash.as_ref() else {
            continue;
        };
        let entry = totals
            .entry((
                record_hash.clone(),
                fact.currency.clone(),
                fact.channel.clone(),
            ))
            .or_default();
        if fact.direction == "import" {
            entry.0 += fact.account_value;
        } else if fact.direction == "export" {
            entry.1 += fact.account_value;
        }
    }
    let mut points = totals
        .into_iter()
        .filter_map(|((record_hash, currency, channel), (imports, exports))| {
            let record = record_by_hash.get(record_hash.as_str())?;
            Some(MarketTradePoint {
                record_hash,
                year: record.year,
                day: record.day,
                game_day: record.game_day,
                currency,
                channel,
                import_value: imports,
                export_value: exports,
                trade_result: exports - imports,
            })
        })
        .collect::<Vec<_>>();
    points.sort_by_key(|point| {
        (
            point.game_day,
            point.currency.clone(),
            point.channel.clone(),
        )
    });
    points
}

fn resource_ledger(
    facts: &[&crate::model::MarketWarehouseTradeFact],
) -> Vec<MarketResourceLedgerRow> {
    let mut rows = BTreeMap::<(String, String, String), MarketResourceLedgerRow>::new();
    for fact in facts {
        let row = rows
            .entry((
                fact.currency.clone(),
                fact.channel.clone(),
                fact.resource_token.clone(),
            ))
            .or_insert_with(|| MarketResourceLedgerRow {
                currency: fact.currency.clone(),
                channel: fact.channel.clone(),
                resource_token: fact.resource_token.clone(),
                import_quantity: 0.0,
                export_quantity: 0.0,
                import_account_value: 0.0,
                export_account_value: 0.0,
                trade_result: 0.0,
                disposal_cost: None,
                source_fields: Vec::new(),
            });
        if fact.direction == "import" {
            row.import_quantity += fact.quantity;
            row.import_account_value += fact.account_value;
        } else if fact.direction == "export" {
            row.export_quantity += fact.quantity;
            row.export_account_value += fact.account_value;
            if fact.account_value < 0.0 {
                row.disposal_cost =
                    Some(row.disposal_cost.unwrap_or_default() + -fact.account_value);
            }
        }
        if !row.source_fields.contains(&fact.source_field) {
            row.source_fields.push(fact.source_field.clone());
        }
        row.trade_result = row.export_account_value - row.import_account_value;
    }
    rows.into_values().collect()
}

fn selected_head_facts<'a, T>(
    facts: &'a [T],
    latest_record_hash: Option<&str>,
    scope_kind: impl Fn(&T) -> Option<&str>,
    record_hash: impl Fn(&T) -> Option<&str>,
) -> Vec<&'a T> {
    let has_republic_snapshot = facts
        .iter()
        .any(|fact| scope_kind(fact) == Some("republic"));
    facts
        .iter()
        .filter(|fact| {
            if has_republic_snapshot {
                scope_kind(fact) == Some("republic")
            } else {
                scope_kind(fact).is_none() && record_hash(fact) == latest_record_hash
            }
        })
        .collect()
}

fn currency_pulses(
    projection: &MarketWarehouseProjection,
    ledger: &[MarketResourceLedgerRow],
) -> Vec<MarketCurrencyPulse> {
    ["rub", "usd"]
        .into_iter()
        .map(|currency| {
            let channel_total = |channel: &str, export: bool| {
                ledger
                    .iter()
                    .filter(|row| row.currency == currency && row.channel == channel)
                    .map(|row| {
                        if export {
                            row.export_account_value
                        } else {
                            row.import_account_value
                        }
                    })
                    .sum::<f64>()
            };
            let standard_import = channel_total("standard", false);
            let standard_export = channel_total("standard", true);
            let international_import = channel_total("international", false);
            let international_export = channel_total("international", true);
            let positive = ledger
                .iter()
                .filter(|row| {
                    row.currency == currency
                        && row.channel == "standard"
                        && row.export_account_value > 0.0
                })
                .map(|row| row.export_account_value)
                .collect::<Vec<_>>();
            let positive_total = positive.iter().sum::<f64>();
            let hhi = (positive_total > 0.0).then(|| {
                positive
                    .iter()
                    .map(|value| (value / positive_total).powi(2))
                    .sum::<f64>()
            });
            MarketCurrencyPulse {
                currency: currency.to_owned(),
                standard_import_value: standard_import,
                standard_export_value: standard_export,
                standard_trade_result: standard_export - standard_import,
                international_import_value: international_import,
                international_export_value: international_export,
                international_trade_result: international_export - international_import,
                positive_export_hhi: hhi,
                positive_export_resource_count: positive.len() as u32,
                context: MarketMetricContext {
                    metric_id: format!("market.trade_result.{currency}"),
                    formula: "export_account_value - import_account_value".to_owned(),
                    currency: Some(currency.to_owned()),
                    unit: "source_currency_account_value".to_owned(),
                    time_basis: "selected_head_source_window".to_owned(),
                    exclusions: vec![
                        "channels_separate".to_owned(),
                        "negative_exports_are_disposal".to_owned(),
                        "no_annualisation_or_interpolation".to_owned(),
                    ],
                    evidence_class: projection.mapping_classification.clone(),
                    profile_id: projection.profile_id.clone(),
                    profile_version: projection.profile_version.clone(),
                    source_fields: ledger
                        .iter()
                        .filter(|row| row.currency == currency)
                        .flat_map(|row| row.source_fields.clone())
                        .collect(),
                    analytical_head: projection.interpretation_id.clone(),
                },
            }
        })
        .collect()
}

fn price_ledger(
    projection: &MarketWarehouseProjection,
    latest_record_hash: Option<&str>,
) -> Vec<MarketPriceLedgerRow> {
    let latest = selected_head_facts(
        &projection.prices,
        latest_record_hash,
        |fact| fact.scope_kind.as_deref(),
        |fact| fact.record_hash.as_deref(),
    );
    let first_record = projection
        .records
        .iter()
        .min_by_key(|record| record.ordinal);
    let mut rows = BTreeMap::<(String, String), MarketPriceLedgerRow>::new();
    for fact in latest {
        let row = rows
            .entry((fact.currency.clone(), fact.resource_token.clone()))
            .or_insert_with(|| MarketPriceLedgerRow {
                currency: fact.currency.clone(),
                resource_token: fact.resource_token.clone(),
                purchase_price: None,
                sell_price: None,
                base_price: None,
                purchase_index: None,
                sell_index: None,
                robust_log_volatility: None,
                volatility_observations: 0,
                source_fields: Vec::new(),
            });
        match fact.price_side.as_str() {
            "purchase" => row.purchase_price = Some(fact.value),
            "sell" => row.sell_price = Some(fact.value),
            "base" => row.base_price = Some(fact.value),
            _ => {}
        }
        if !row.source_fields.contains(&fact.source_field) {
            row.source_fields.push(fact.source_field.clone());
        }
    }
    for row in rows.values_mut() {
        let base = first_record.and_then(|record| {
            projection.prices.iter().find(|fact| {
                fact.record_hash.as_deref() == Some(record.record_hash.as_str())
                    && fact.currency == row.currency
                    && fact.resource_token == row.resource_token
                    && fact.price_side == "purchase"
            })
        });
        row.purchase_index = ratio_index(row.purchase_price, base.map(|fact| fact.value));
        let sell_base = first_record.and_then(|record| {
            projection.prices.iter().find(|fact| {
                fact.record_hash.as_deref() == Some(record.record_hash.as_str())
                    && fact.currency == row.currency
                    && fact.resource_token == row.resource_token
                    && fact.price_side == "sell"
            })
        });
        row.sell_index = ratio_index(row.sell_price, sell_base.map(|fact| fact.value));
        if let Some(volatility) = projection.analytical_price_volatility.iter().find(|value| {
            value.currency == row.currency && value.resource_token == row.resource_token
        }) {
            row.volatility_observations = volatility.observations;
            row.robust_log_volatility = Some(volatility.robust_log_volatility);
        } else {
            let prices = projection
                .records
                .iter()
                .filter_map(|record| {
                    projection.prices.iter().find(|fact| {
                        fact.record_hash.as_deref() == Some(record.record_hash.as_str())
                            && fact.currency == row.currency
                            && fact.resource_token == row.resource_token
                            && fact.price_side == "purchase"
                            && fact.value > 0.0
                    })
                })
                .map(|fact| fact.value)
                .collect::<Vec<_>>();
            let movements = prices
                .windows(2)
                .map(|window| (window[1] / window[0]).ln())
                .filter(|value| value.is_finite())
                .collect::<Vec<_>>();
            row.volatility_observations = movements.len() as u32;
            row.robust_log_volatility = median_absolute_deviation(&movements);
        }
    }
    rows.into_values().collect()
}

fn ratio_index(current: Option<f64>, base: Option<f64>) -> Option<f64> {
    match (current, base) {
        (Some(current), Some(base)) if base > 0.0 => Some(current / base * 100.0),
        _ => None,
    }
}

fn median_absolute_deviation(values: &[f64]) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }
    let median_value = median(values.to_vec())?;
    let deviations = values
        .iter()
        .map(|value| (value - median_value).abs())
        .collect::<Vec<_>>();
    median(deviations).map(|value| value * 1.4826)
}

fn median(mut values: Vec<f64>) -> Option<f64> {
    values.retain(|value| value.is_finite());
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    Some(if values.len().is_multiple_of(2) {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    })
}

fn scalar_ledger(
    projection: &MarketWarehouseProjection,
    latest_record_hash: Option<&str>,
) -> Vec<MarketScalarLedgerRow> {
    selected_head_facts(
        &projection.scalars,
        latest_record_hash,
        |fact| fact.scope_kind.as_deref(),
        |fact| fact.record_hash.as_deref(),
    )
    .into_iter()
    .map(|fact| MarketScalarLedgerRow {
        fact_id: fact.fact_id.clone(),
        currency: fact.currency.clone(),
        category: fact.category,
        value: fact.value,
        source_field: fact.source_field.clone(),
        source_line: fact.source_line,
    })
    .collect()
}

fn city_ledger(projection: &MarketWarehouseProjection) -> Vec<MarketCityRow> {
    let mut cities = BTreeMap::<(String, String, String), (f64, f64)>::new();
    for fact in projection
        .trades
        .iter()
        .filter(|fact| fact.scope_kind.as_deref() == Some("city"))
    {
        let Some(scope_id) = fact.scope_id.as_ref() else {
            continue;
        };
        let entry = cities
            .entry((
                scope_id.clone(),
                fact.currency.clone(),
                fact.channel.clone(),
            ))
            .or_default();
        if fact.direction == "import" {
            entry.0 += fact.account_value;
        } else if fact.direction == "export" {
            entry.1 += fact.account_value;
        }
    }
    cities
        .into_iter()
        .map(
            |((source_id, currency, channel), (imports, exports))| MarketCityRow {
                source_id,
                currency,
                channel,
                import_value: imports,
                export_value: exports,
                trade_result: exports - imports,
            },
        )
        .collect()
}

fn builtin_baskets(
    projection: &MarketWarehouseProjection,
    base_record_hash: Option<&str>,
) -> Vec<MarketBasketSummary> {
    let mut baskets = Vec::new();
    let first_record_hash = projection
        .records
        .iter()
        .min_by_key(|record| record.ordinal)
        .map(|record| record.record_hash.as_str())
        .or(base_record_hash);
    for currency in ["rub", "usd"] {
        let import_weights = projection
            .trades
            .iter()
            .filter(|row| {
                row.record_hash.as_deref() == first_record_hash
                    && row.currency == currency
                    && row.channel == "standard"
                    && row.direction == "import"
                    && row.quantity > 0.0
                    && row.account_value > 0.0
            })
            .fold(BTreeMap::<String, f64>::new(), |mut weights, row| {
                *weights.entry(row.resource_token.clone()).or_default() += row.quantity;
                weights
            })
            .into_iter()
            .map(
                |(resource_token, weight)| crate::model::MarketBasketWeight {
                    resource_token,
                    weight,
                },
            )
            .collect::<Vec<_>>();
        let export_weights = projection
            .trades
            .iter()
            .filter(|row| {
                row.record_hash.as_deref() == first_record_hash
                    && row.currency == currency
                    && row.channel == "standard"
                    && row.direction == "export"
                    && row.quantity > 0.0
                    && row.account_value > 0.0
            })
            .fold(BTreeMap::<String, f64>::new(), |mut weights, row| {
                *weights.entry(row.resource_token.clone()).or_default() += row.quantity;
                weights
            })
            .into_iter()
            .map(
                |(resource_token, weight)| crate::model::MarketBasketWeight {
                    resource_token,
                    weight,
                },
            )
            .collect::<Vec<_>>();
        let mut imports = MarketBasketSummary {
            basket_id: format!("builtin.observed-imports.{currency}"),
            revision: 1,
            name: "observed_imports".to_owned(),
            currency: currency.to_owned(),
            price_side: "purchase".to_owned(),
            built_in: true,
            selected: false,
            base_record_hash: first_record_hash.map(str::to_owned),
            resource_count: import_weights.len() as u32,
            coverage_resources: 0,
            index_value: None,
            reason: "observed_positive_import_quantities".to_owned(),
            weights: import_weights,
        };
        evaluate_basket(&mut imports, projection, base_record_hash);
        baskets.push(imports);
        let mut exports = MarketBasketSummary {
            basket_id: format!("builtin.observed-positive-exports.{currency}"),
            revision: 1,
            name: "observed_positive_exports".to_owned(),
            currency: currency.to_owned(),
            price_side: "sell".to_owned(),
            built_in: true,
            selected: false,
            base_record_hash: first_record_hash.map(str::to_owned),
            resource_count: export_weights.len() as u32,
            coverage_resources: 0,
            index_value: None,
            reason: "observed_positive_export_quantities_excluding_disposal".to_owned(),
            weights: export_weights,
        };
        evaluate_basket(&mut exports, projection, base_record_hash);
        baskets.push(exports);
    }
    baskets
}

fn evaluate_basket(
    basket: &mut MarketBasketSummary,
    projection: &MarketWarehouseProjection,
    latest_record_hash: Option<&str>,
) {
    let Some(base_record_hash) = basket.base_record_hash.as_deref() else {
        return;
    };
    let Some(latest_record_hash) = latest_record_hash else {
        return;
    };
    let mut base_total = 0.0;
    let mut current_total = 0.0;
    let mut coverage = 0_u32;
    for weight in &basket.weights {
        let price = |record_hash: &str| {
            projection.prices.iter().find(|fact| {
                fact.record_hash.as_deref() == Some(record_hash)
                    && fact.currency == basket.currency
                    && fact.price_side == basket.price_side
                    && fact.resource_token == weight.resource_token
                    && fact.value.is_finite()
                    && fact.value >= 0.0
            })
        };
        let (Some(base), Some(current)) = (price(base_record_hash), price(latest_record_hash))
        else {
            continue;
        };
        base_total += weight.weight * base.value;
        current_total += weight.weight * current.value;
        coverage = coverage.saturating_add(1);
    }
    basket.coverage_resources = coverage;
    basket.index_value = (base_total > 0.0).then_some(current_total / base_total * 100.0);
}

fn metric_contexts(projection: &MarketWarehouseProjection) -> Vec<MarketMetricContext> {
    let context = |metric_id: String,
                   formula: &str,
                   currency: Option<&str>,
                   unit: &str,
                   time_basis: &str,
                   exclusions: Vec<&str>,
                   source_fields: Vec<String>| MarketMetricContext {
        metric_id,
        formula: formula.to_owned(),
        currency: currency.map(str::to_owned),
        unit: unit.to_owned(),
        time_basis: time_basis.to_owned(),
        exclusions: exclusions.into_iter().map(str::to_owned).collect(),
        evidence_class: projection.mapping_classification.clone(),
        profile_id: projection.profile_id.clone(),
        profile_version: projection.profile_version.clone(),
        source_fields,
        analytical_head: projection.interpretation_id.clone(),
    };
    let mut contexts = Vec::new();
    for currency in ["rub", "usd"] {
        let price_fields = projection
            .prices
            .iter()
            .filter(|fact| fact.currency == currency)
            .map(|fact| fact.source_field.clone())
            .collect::<Vec<_>>();
        let trade_fields = projection
            .trades
            .iter()
            .filter(|fact| fact.currency == currency)
            .map(|fact| fact.source_field.clone())
            .collect::<Vec<_>>();
        contexts.push(context(
            format!("market.positive_export_hhi.{currency}.standard"),
            "positive_export_hhi",
            Some(currency),
            "concentration_index",
            "selected_head_source_window",
            vec!["standard_channel_only", "non_positive_exports_excluded"],
            trade_fields.clone(),
        ));
        contexts.push(context(
            format!("market.price.{currency}"),
            "recorded_price_and_relative_index",
            Some(currency),
            "source_currency_per_resource_unit",
            "selected_head_and_first_compatible_record",
            vec!["no_annualisation_or_interpolation"],
            price_fields.clone(),
        ));
        contexts.push(context(
            format!("market.price_volatility.{currency}"),
            "robust_log_price_movement",
            Some(currency),
            "log_price_movement",
            "available_proven_history_through_selected_head",
            vec!["positive_prices_only", "no_annualisation_or_interpolation"],
            price_fields,
        ));
        contexts.push(context(
            format!("market.city_trade_result.{currency}.standard"),
            "export_account_value - import_account_value",
            Some(currency),
            "source_currency_account_value",
            "selected_head_city_snapshot",
            vec!["standard_channel_only", "city_republic_windows_separate"],
            trade_fields,
        ));
    }
    contexts.push(context(
        "market.scalar_accounts".to_owned(),
        "recorded_source_value",
        None,
        "source_native",
        "selected_head_source_window",
        vec!["compatible_denominator_required"],
        projection
            .scalars
            .iter()
            .map(|fact| fact.source_field.clone())
            .collect(),
    ));
    for context in &mut contexts {
        context.source_fields.sort();
        context.source_fields.dedup();
    }
    contexts
}

fn terms_of_trade(
    baskets: &[MarketBasketSummary],
    projection: &MarketWarehouseProjection,
) -> Vec<MarketTermsOfTradeSummary> {
    let mut results = Vec::new();
    for imports in baskets
        .iter()
        .filter(|basket| basket.price_side == "purchase")
    {
        let (Some(base_record_hash), Some(import_index)) =
            (imports.base_record_hash.as_deref(), imports.index_value)
        else {
            continue;
        };
        for exports in baskets.iter().filter(|basket| {
            basket.price_side == "sell"
                && basket.currency == imports.currency
                && basket.base_record_hash.as_deref() == Some(base_record_hash)
        }) {
            let Some(export_index) = exports.index_value else {
                continue;
            };
            if import_index <= 0.0 {
                continue;
            }
            results.push(MarketTermsOfTradeSummary {
                currency: imports.currency.clone(),
                base_record_hash: base_record_hash.to_owned(),
                import_basket_id: imports.basket_id.clone(),
                import_basket_revision: imports.revision,
                export_basket_id: exports.basket_id.clone(),
                export_basket_revision: exports.revision,
                import_index,
                export_index,
                terms_of_trade_index: export_index / import_index * 100.0,
                context: MarketMetricContext {
                    metric_id: format!("market.terms_of_trade.{}", imports.currency),
                    formula: "export_price_index / import_price_index * 100".to_owned(),
                    currency: Some(imports.currency.clone()),
                    unit: "index_base_100".to_owned(),
                    time_basis: "matched_baskets_same_base_record".to_owned(),
                    exclusions: vec![
                        "currencies_separate".to_owned(),
                        "same_base_record_required".to_owned(),
                    ],
                    evidence_class: projection.mapping_classification.clone(),
                    profile_id: projection.profile_id.clone(),
                    profile_version: projection.profile_version.clone(),
                    source_fields: projection
                        .prices
                        .iter()
                        .filter(|fact| fact.currency == imports.currency)
                        .map(|fact| fact.source_field.clone())
                        .collect(),
                    analytical_head: projection.interpretation_id.clone(),
                },
            });
            if results.len() >= 64 {
                return results;
            }
        }
    }
    results
}

fn limitations() -> Vec<String> {
    vec![
        "reserves_unavailable".to_owned(),
        "city_republic_windows_separate".to_owned(),
        "currencies_require_explicit_exchange".to_owned(),
        "loan_tourism_denominator_required".to_owned(),
        "no_annualisation_or_interpolation".to_owned(),
    ]
}

fn evaluate_scenarios(
    scenarios: Vec<crate::model::MarketScenarioSummary>,
    currencies: &[MarketCurrencyPulse],
    scalars: &[MarketScalarLedgerRow],
) -> Vec<crate::model::MarketScenarioSummary> {
    scenarios
        .into_iter()
        .map(|mut summary| {
            let Ok(draft) = serde_json::from_str::<crate::model::MarketScenarioDraft>(
                &summary.assumptions_json,
            ) else {
                return summary;
            };
            match draft.scenario_kind.as_str() {
                "break_even" => {
                    let (Some(domestic), Some(delivery), Some(efficiency)) = (
                        draft.domestic_unit_cost,
                        draft.delivery_cost,
                        draft.operating_efficiency_percent,
                    ) else {
                        return summary;
                    };
                    summary.result_kind = Some("break_even_unit_cost".to_owned());
                    summary.result_value = Some((domestic + delivery) / (efficiency / 100.0));
                    summary.result_unit = Some(format!("{}_per_source_unit", draft.currency));
                    summary.covered_components = 3;
                }
                "debt_stress" => {
                    let Some(debt_service) = draft.debt_service else {
                        return summary;
                    };
                    let Some(pulse) = currencies
                        .iter()
                        .find(|pulse| pulse.currency == draft.currency)
                    else {
                        return summary;
                    };
                    let export_factor =
                        1.0 - draft.export_stress_percent.unwrap_or_default() / 100.0;
                    let tourism_factor =
                        1.0 - draft.tourism_stress_percent.unwrap_or_default() / 100.0;
                    let mut income = 0.0;
                    let mut covered = 0_u32;
                    for component in &draft.included_income_components {
                        let value = match component.as_str() {
                            "standard_exports" => {
                                Some(pulse.standard_export_value.max(0.0) * export_factor)
                            }
                            "international_exports" => {
                                Some(pulse.international_export_value.max(0.0) * export_factor)
                            }
                            "tourism_spend" => scalars
                                .iter()
                                .find(|fact| {
                                    fact.fact_id == "market.tourism.spend"
                                        && fact.currency.as_deref() == Some(draft.currency.as_str())
                                })
                                .map(|fact| fact.value.max(0.0) * tourism_factor),
                            _ => None,
                        };
                        if let Some(value) = value {
                            income += value;
                            covered = covered.saturating_add(1);
                        }
                    }
                    summary.result_kind = Some("debt_service_coverage".to_owned());
                    summary.result_value = (debt_service > 0.0).then_some(income / debt_service);
                    summary.result_unit = Some("coverage_ratio".to_owned());
                    summary.covered_components = covered;
                }
                _ => {}
            }
            summary
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        currency_pulses, evaluate_scenarios, median_absolute_deviation, ratio_index,
        resource_ledger, selected_head_facts, terms_of_trade,
    };
    use crate::model::{
        MarketBasketSummary, MarketScenarioDraft, MarketScenarioSummary, MarketWarehouseProjection,
        MarketWarehouseTradeFact,
    };

    #[derive(Debug)]
    struct Fact {
        scope: Option<&'static str>,
        record: Option<&'static str>,
        value: u32,
    }

    #[test]
    fn price_indices_require_a_positive_base() {
        assert_eq!(ratio_index(Some(15.0), Some(10.0)), Some(150.0));
        assert_eq!(ratio_index(Some(15.0), Some(0.0)), None);
    }

    #[test]
    fn robust_volatility_uses_log_movement_median_deviation() {
        let value = median_absolute_deviation(&[0.0, 0.1, 0.2]).expect("volatility");
        assert!((value - 0.14826).abs() < 0.00001);
    }

    #[test]
    fn exact_republic_snapshot_does_not_double_count_the_latest_history_record() {
        let facts = vec![
            Fact {
                scope: None,
                record: Some("older"),
                value: 1,
            },
            Fact {
                scope: None,
                record: Some("latest"),
                value: 2,
            },
            Fact {
                scope: Some("republic"),
                record: None,
                value: 3,
            },
            Fact {
                scope: Some("city"),
                record: None,
                value: 4,
            },
        ];
        let selected = selected_head_facts(
            &facts,
            Some("latest"),
            |fact| fact.scope,
            |fact| fact.record,
        );
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].value, 3);
    }

    #[test]
    fn latest_history_record_is_used_when_no_republic_snapshot_exists() {
        let facts = vec![
            Fact {
                scope: None,
                record: Some("older"),
                value: 1,
            },
            Fact {
                scope: None,
                record: Some("latest"),
                value: 2,
            },
        ];
        let selected = selected_head_facts(
            &facts,
            Some("latest"),
            |fact| fact.scope,
            |fact| fact.record,
        );
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].value, 2);
    }

    fn projection(trades: Vec<MarketWarehouseTradeFact>) -> MarketWarehouseProjection {
        MarketWarehouseProjection {
            interpretation_id: "interpretation".to_owned(),
            raw_payload_hash: "raw".to_owned(),
            branch_id: "main".to_owned(),
            profile_id: "reviewed".to_owned(),
            profile_version: "1.1.0".to_owned(),
            resolved_profile_hash: "resolved".to_owned(),
            mapping_classification: "reviewed_mapping".to_owned(),
            records: Vec::new(),
            prices: Vec::new(),
            trades,
            scalars: Vec::new(),
            analytical_trade_history: Vec::new(),
            analytical_price_volatility: Vec::new(),
        }
    }

    fn trade(
        currency: &str,
        direction: &str,
        resource: &str,
        quantity: f64,
        account_value: f64,
    ) -> MarketWarehouseTradeFact {
        MarketWarehouseTradeFact {
            record_hash: Some("record".to_owned()),
            scope_kind: None,
            scope_id: None,
            currency: currency.to_owned(),
            direction: direction.to_owned(),
            channel: "standard".to_owned(),
            resource_token: resource.to_owned(),
            quantity,
            account_value,
            source_field: format!("${direction}_{currency}"),
            source_line: 1,
            mapping_id: format!("market.trade.{direction}.standard.{currency}"),
        }
    }

    #[test]
    fn trade_results_keep_currencies_separate_and_classify_negative_exports_as_disposal() {
        let facts = vec![
            trade("rub", "import", "waste", 3.0, 10.0),
            trade("rub", "export", "waste", 1.0, -2.0),
            trade("usd", "export", "steel", 1.0, 5.0),
        ];
        let references = facts.iter().collect::<Vec<_>>();
        let ledger = resource_ledger(&references);
        let rub = ledger
            .iter()
            .find(|row| row.currency == "rub")
            .expect("RUB row");
        assert_eq!(rub.trade_result, -12.0);
        assert_eq!(rub.disposal_cost, Some(2.0));
        let projection = projection(facts);
        let pulses = currency_pulses(&projection, &ledger);
        let rub = pulses
            .iter()
            .find(|pulse| pulse.currency == "rub")
            .expect("RUB pulse");
        let usd = pulses
            .iter()
            .find(|pulse| pulse.currency == "usd")
            .expect("USD pulse");
        assert_eq!(rub.standard_trade_result, -12.0);
        assert_eq!(rub.positive_export_hhi, None);
        assert_eq!(usd.standard_trade_result, 5.0);
        assert_eq!(usd.positive_export_hhi, Some(1.0));
    }

    fn basket(id: &str, side: &str, base: &str, index: f64) -> MarketBasketSummary {
        MarketBasketSummary {
            basket_id: id.to_owned(),
            revision: 1,
            name: id.to_owned(),
            currency: "rub".to_owned(),
            price_side: side.to_owned(),
            built_in: false,
            selected: false,
            base_record_hash: Some(base.to_owned()),
            resource_count: 1,
            coverage_resources: 1,
            index_value: Some(index),
            reason: "test".to_owned(),
            weights: Vec::new(),
        }
    }

    #[test]
    fn terms_of_trade_require_matching_currency_and_base_record() {
        let projection = projection(Vec::new());
        let baskets = vec![
            basket("imports", "purchase", "same", 110.0),
            basket("exports", "sell", "same", 121.0),
            basket("different-base", "sell", "other", 200.0),
        ];
        let result = terms_of_trade(&baskets, &projection);
        assert_eq!(result.len(), 1);
        assert!((result[0].terms_of_trade_index - 110.0).abs() < 1e-9);
    }

    fn scenario_summary(draft: MarketScenarioDraft) -> MarketScenarioSummary {
        MarketScenarioSummary {
            scenario_id: draft.scenario_id.clone(),
            revision: 1,
            name: draft.name.clone(),
            scenario_kind: draft.scenario_kind.clone(),
            reason: draft.reason.clone(),
            assumptions_json: serde_json::to_string(&draft).expect("scenario JSON"),
            selected: true,
            result_kind: None,
            result_value: None,
            result_unit: None,
            covered_components: 0,
        }
    }

    #[test]
    fn player_scenarios_use_only_explicit_same_currency_assumptions() {
        let break_even = scenario_summary(MarketScenarioDraft {
            scenario_id: "break-even".to_owned(),
            name: "Break even".to_owned(),
            scenario_kind: "break_even".to_owned(),
            currency: "rub".to_owned(),
            reason: "test".to_owned(),
            domestic_unit_cost: Some(50.0),
            delivery_cost: Some(10.0),
            operating_efficiency_percent: Some(80.0),
            exchange_rate: None,
            debt_service: None,
            export_stress_percent: None,
            tourism_stress_percent: None,
            included_income_components: Vec::new(),
        });
        let pulses = currency_pulses(
            &projection(Vec::new()),
            &[crate::model::MarketResourceLedgerRow {
                currency: "rub".to_owned(),
                channel: "standard".to_owned(),
                resource_token: "steel".to_owned(),
                import_quantity: 0.0,
                export_quantity: 1.0,
                import_account_value: 0.0,
                export_account_value: 100.0,
                trade_result: 100.0,
                disposal_cost: None,
                source_fields: Vec::new(),
            }],
        );
        let debt = scenario_summary(MarketScenarioDraft {
            scenario_id: "debt".to_owned(),
            name: "Debt".to_owned(),
            scenario_kind: "debt_stress".to_owned(),
            currency: "rub".to_owned(),
            reason: "test".to_owned(),
            domestic_unit_cost: None,
            delivery_cost: None,
            operating_efficiency_percent: None,
            exchange_rate: None,
            debt_service: Some(100.0),
            export_stress_percent: Some(50.0),
            tourism_stress_percent: None,
            included_income_components: vec!["standard_exports".to_owned()],
        });
        let results = evaluate_scenarios(vec![break_even, debt], &pulses, &[]);
        assert_eq!(results[0].result_value, Some(75.0));
        assert_eq!(results[1].result_value, Some(0.5));
    }
}
