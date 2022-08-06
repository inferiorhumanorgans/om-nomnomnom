use crate::*;
use tracing_test::traced_test;

macro_rules! open_metrics_test {
    ($test_name:ident$(, $attrib:ident)?) => {
        #[test]
        #[traced_test]
        $(#[$attrib])?
        fn $test_name() {
            let test_data = include_str!(concat!("../../parse-tests/", stringify!($test_name), "/metrics"));

            let test_meta = include_str!(concat!("../../parse-tests/", stringify!($test_name), "/test.json"));
            let test_meta: serde_json::Value = serde_json::from_str(test_meta).expect("invalid json");
            let should_parse = test_meta["shouldParse"] == serde_json::Value::Bool(true);

            let nom_result = lexer::exposition(test_data);

            if !should_parse && nom_result.is_err() {
                return
            }

            let (_, tokens) = nom_result.expect(stringify!($test_name));

            if test_meta["tokens"].is_null() {
                error!(got=%serde_json::to_string_pretty(&tokens).expect("couldn't serialize json"))
            }
            // assert_eq!(test_meta["tokens"], serde_json::to_value(&tokens).unwrap());

            let parser_result = parser::parse(tokens);

            if !should_parse {
                assert!(parser_result.is_err());
            } else {
                let metric_set : std::collections::HashMap<&str, parser::MetricFamily> = parser_result.expect("couldn't parse tokens");
                info!(expected=%serde_json::to_string_pretty(&metric_set).expect("couldn't serialize json"));
                // assert_eq!(test_meta["parsed"], serde_json::to_value(&metric_set).unwrap());
            }
        }
    }
}

open_metrics_test!(bad_blank_line);
open_metrics_test!(bad_clashing_names_0);
open_metrics_test!(bad_clashing_names_1);
open_metrics_test!(bad_clashing_names_2);
open_metrics_test!(bad_counter_values_0);
open_metrics_test!(bad_counter_values_1);
open_metrics_test!(bad_counter_values_2);
open_metrics_test!(bad_counter_values_3);
open_metrics_test!(bad_counter_values_4);
open_metrics_test!(bad_counter_values_5);
open_metrics_test!(bad_counter_values_6);
open_metrics_test!(bad_counter_values_7);
open_metrics_test!(bad_counter_values_8);
open_metrics_test!(bad_counter_values_9);
open_metrics_test!(bad_counter_values_10);
open_metrics_test!(bad_counter_values_11);
open_metrics_test!(bad_counter_values_12);
open_metrics_test!(bad_counter_values_13);
open_metrics_test!(bad_counter_values_14);
open_metrics_test!(bad_counter_values_15);
open_metrics_test!(bad_counter_values_16);
open_metrics_test!(bad_counter_values_17);
open_metrics_test!(bad_counter_values_18);
open_metrics_test!(bad_counter_values_19);
open_metrics_test!(bad_exemplar_complex_chars);
open_metrics_test!(bad_exemplar_timestamp_0);
open_metrics_test!(bad_exemplar_timestamp_1);
open_metrics_test!(bad_exemplar_timestamp_2);
open_metrics_test!(bad_exemplars_0);
open_metrics_test!(bad_exemplars_1);
open_metrics_test!(bad_exemplars_2);
open_metrics_test!(bad_exemplars_3);
open_metrics_test!(bad_exemplars_4);
open_metrics_test!(bad_exemplars_5);
open_metrics_test!(bad_exemplars_6);
open_metrics_test!(bad_exemplars_7);
open_metrics_test!(bad_exemplars_8);
open_metrics_test!(bad_exemplars_9);
open_metrics_test!(bad_exemplars_10);
open_metrics_test!(bad_exemplars_11);
open_metrics_test!(bad_exemplars_12);
open_metrics_test!(bad_exemplars_on_unallowed_metric_types_0);
open_metrics_test!(bad_exemplars_on_unallowed_metric_types_1);
open_metrics_test!(bad_exemplars_on_unallowed_metric_types_2);
open_metrics_test!(bad_exemplars_on_unallowed_samples_0);
open_metrics_test!(bad_exemplars_on_unallowed_samples_1);
open_metrics_test!(bad_exemplars_on_unallowed_samples_2);
open_metrics_test!(bad_exemplars_on_unallowed_samples_3);
open_metrics_test!(bad_grouping_or_ordering_0);
open_metrics_test!(bad_grouping_or_ordering_1);
open_metrics_test!(bad_grouping_or_ordering_2);
open_metrics_test!(bad_grouping_or_ordering_3);
open_metrics_test!(bad_grouping_or_ordering_4);
open_metrics_test!(bad_grouping_or_ordering_5);
open_metrics_test!(bad_grouping_or_ordering_6);
open_metrics_test!(bad_grouping_or_ordering_7);
open_metrics_test!(bad_grouping_or_ordering_8);
open_metrics_test!(bad_grouping_or_ordering_9);
open_metrics_test!(bad_grouping_or_ordering_10);
open_metrics_test!(bad_help_0);
open_metrics_test!(bad_help_1);
open_metrics_test!(bad_help_2);
open_metrics_test!(bad_help_3);
open_metrics_test!(bad_help_4);
open_metrics_test!(bad_histograms_0);

// If present, the MetricPoint's Created Value Sample MetricName MUST have the suffix
// "_created". If and only if a Sum Value is present in a MetricPoint, then the MetricPoint's
// +Inf Bucket value MUST also appear in a Sample with a MetricName with the suffix "_count".
open_metrics_test!(bad_histograms_1);
open_metrics_test!(bad_histograms_2);

open_metrics_test!(bad_histograms_3);
open_metrics_test!(bad_histograms_4);
open_metrics_test!(bad_histograms_5);
open_metrics_test!(bad_histograms_6);
open_metrics_test!(bad_histograms_7);
open_metrics_test!(bad_histograms_8);

// Buckets MUST be sorted in number increasing order of "le", and the value of the "le" label
// MUST follow the rules for Canonical Numbers.
open_metrics_test!(bad_histograms_9);

open_metrics_test!(bad_histograms_10);
open_metrics_test!(bad_histograms_11);
open_metrics_test!(bad_histograms_12);
open_metrics_test!(bad_histograms_13);
open_metrics_test!(bad_histograms_14);
open_metrics_test!(bad_info_and_stateset_values_0);
open_metrics_test!(bad_info_and_stateset_values_1);
open_metrics_test!(bad_invalid_labels_0);
open_metrics_test!(bad_invalid_labels_1);
open_metrics_test!(bad_invalid_labels_2);
open_metrics_test!(bad_invalid_labels_3);
open_metrics_test!(bad_invalid_labels_4);
open_metrics_test!(bad_invalid_labels_5);
open_metrics_test!(bad_invalid_labels_6);
open_metrics_test!(bad_invalid_labels_7);
open_metrics_test!(bad_invalid_labels_8);
open_metrics_test!(bad_metadata);
open_metrics_test!(bad_metadata_in_wrong_place_0);
open_metrics_test!(bad_metadata_in_wrong_place_1);
open_metrics_test!(bad_metadata_in_wrong_place_2);
open_metrics_test!(bad_metric_names_0);
open_metrics_test!(bad_metric_names_1);
open_metrics_test!(bad_metric_names_2);
open_metrics_test!(bad_missing_equal_or_label_value_0);
open_metrics_test!(bad_missing_equal_or_label_value_1);
open_metrics_test!(bad_missing_equal_or_label_value_2);
open_metrics_test!(bad_missing_equal_or_label_value_3);
open_metrics_test!(bad_missing_equal_or_label_value_4);
open_metrics_test!(bad_missing_or_extra_commas_0);
open_metrics_test!(bad_missing_or_extra_commas_1);
open_metrics_test!(bad_missing_or_extra_commas_2);

// A Summary MetricPoint MAY consist of a Count, Sum, Created, and a set of quantiles.
open_metrics_test!(bad_missing_or_invalid_labels_for_a_type_0);

open_metrics_test!(bad_missing_or_invalid_labels_for_a_type_1);
open_metrics_test!(bad_missing_or_invalid_labels_for_a_type_2);
open_metrics_test!(bad_missing_or_invalid_labels_for_a_type_3);
open_metrics_test!(bad_missing_or_invalid_labels_for_a_type_4);
open_metrics_test!(bad_missing_or_invalid_labels_for_a_type_5);
open_metrics_test!(bad_missing_or_invalid_labels_for_a_type_7);
open_metrics_test!(bad_missing_or_wrong_quotes_on_label_value_0);
open_metrics_test!(bad_missing_or_wrong_quotes_on_label_value_1);
open_metrics_test!(bad_missing_or_wrong_quotes_on_label_value_2);
open_metrics_test!(bad_missing_value_0);
open_metrics_test!(bad_missing_value_1);
open_metrics_test!(bad_no_eof);
open_metrics_test!(bad_repeated_metadata_0);
open_metrics_test!(bad_repeated_metadata_1);
open_metrics_test!(bad_repeated_metadata_2);
open_metrics_test!(bad_repeated_metadata_3);
open_metrics_test!(bad_stateset_info_values_0);
open_metrics_test!(bad_stateset_info_values_1);
open_metrics_test!(bad_stateset_info_values_2);
open_metrics_test!(bad_stateset_info_values_3);
open_metrics_test!(bad_text_after_eof_0);
open_metrics_test!(bad_text_after_eof_1);
open_metrics_test!(bad_timestamp_1);
open_metrics_test!(bad_timestamp_2);
open_metrics_test!(bad_timestamp_3);
open_metrics_test!(bad_timestamp_4);
open_metrics_test!(bad_timestamp_5);
open_metrics_test!(bad_timestamp_6);
open_metrics_test!(bad_timestamp_7);
open_metrics_test!(bad_timestamp_8);
open_metrics_test!(bad_type_0);
open_metrics_test!(bad_type_1);
open_metrics_test!(bad_type_2);
open_metrics_test!(bad_type_3);
open_metrics_test!(bad_type_4);
open_metrics_test!(bad_type_5);
open_metrics_test!(bad_type_6);
open_metrics_test!(bad_type_7);
open_metrics_test!(bad_value_0);
open_metrics_test!(bad_value_1);
open_metrics_test!(bad_value_2);
open_metrics_test!(bad_value_3);
open_metrics_test!(bad_value_4);
open_metrics_test!(bad_value_5);
open_metrics_test!(bad_value_6);
open_metrics_test!(bad_value_7);
open_metrics_test!(bad_value_8);
open_metrics_test!(bad_value_9);
open_metrics_test!(bad_value_10);
open_metrics_test!(bad_value_11);
open_metrics_test!(bad_value_12);

open_metrics_test!(counter_exemplars);
open_metrics_test!(counter_exemplars_empty_brackets);
open_metrics_test!(counter_unit);
open_metrics_test!(duplicate_timestamps_0);
open_metrics_test!(duplicate_timestamps_1);
open_metrics_test!(empty_brackets);
open_metrics_test!(empty_help);
open_metrics_test!(empty_label);

// https://github.com/OpenObservability/OpenMetrics/issues/252
open_metrics_test!(empty_metadata);

open_metrics_test!(escaping);

// The combined length of the label names and values of an Exemplar's LabelSet MUST NOT exceed 128 UTF-8 character code points.
// …
// There is a hard 128 UTF-8 character limit on exemplar length, to prevent misuse of the feature for tracing span data and other event logging.
// Disable this by setting the feature "naive_wide_char_support" 🙄
open_metrics_test!(exemplars_wide_chars);

open_metrics_test!(exemplars_with_hash_in_label_values);
open_metrics_test!(float_gauge);
open_metrics_test!(gaugehistogram_exemplars);
open_metrics_test!(hash_in_label_value);
open_metrics_test!(help_escaping);
open_metrics_test!(histogram_exemplars);
open_metrics_test!(histogram_noncanonical);
open_metrics_test!(info_timestamps);
open_metrics_test!(label_escaping);
open_metrics_test!(labels_and_infinite);
open_metrics_test!(labels_with_curly_braces);
open_metrics_test!(leading_zeros_float_gauge);
open_metrics_test!(leading_zeros_simple_gauge);
open_metrics_test!(nan);
open_metrics_test!(nan_gauge);
open_metrics_test!(negative_bucket_gaugehistogram);
open_metrics_test!(negative_bucket_histogram);
open_metrics_test!(no_metadata);
open_metrics_test!(no_newline_after_eof);
open_metrics_test!(null_byte);
open_metrics_test!(roundtrip);
open_metrics_test!(simple_counter);
open_metrics_test!(simple_gauge);
open_metrics_test!(simple_gaugehistogram);
open_metrics_test!(simple_histogram);
open_metrics_test!(simple_stateset);
open_metrics_test!(simple_summary);
open_metrics_test!(summary_quantiles);
open_metrics_test!(timestamps);
open_metrics_test!(uint64_counter);
open_metrics_test!(unit_gauge);
open_metrics_test!(untyped);
