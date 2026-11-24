/**
 * DMN TCK test cases that are known to fail.
 *
 * The engine evaluates FEEL with feelin (https://github.com/nikku/feelin).
 * Most known failures are gaps in feelin; each group names the gap and links
 * the feelin pull request that closes it, where one exists. The pull requests
 * were checked by running the TCK against each of them.
 *
 * Tests are listed by `<folder>/<test file>` and test case id. They apply to
 * every DMN version the TCK runs as.
 *
 * @type { Array<{ reason: string, links?: string[], tests: Record<string, string[]> }> }
 */
const KNOWN_FAILURES = [
  {
    reason: 'feelin yields `null` without reporting an error',
    links: [ 'https://github.com/barmac/feelin/pull/12' ],
    tests: {
      '0050-feel-abs-function/0050-feel-abs-function-test-01': [
        '004', '008'
      ],
      '0051-feel-sqrt-function/0051-feel-sqrt-function-test-01': [
        '004', '008', '009', '010', '011', '012', '013', '014', '015'
      ],
      '0052-feel-exp-function/0052-feel-exp-function-test-01': [
        '004', '008', '009', '010', '011', '012', '013', '014', '015'
      ],
      '0053-feel-log-function/0053-feel-log-function-test-01': [
        '004', '008', '009', '010', '011', '012', '013', '014', '015'
      ],
      '0054-feel-even-function/0054-feel-even-function-test-01': [
        '004', '008', '009', '010', '011', '012', '013', '014', '015'
      ],
      '0055-feel-odd-function/0055-feel-odd-function-test-01': [
        '004', '008', '009', '010', '011', '012', '013', '014', '015'
      ],
      '0056-feel-modulo-function/0056-feel-modulo-function-test-01': [
        '004', '005', '008', '008_a', '008_b', '009', '010', '011', '012', '013', '014', '015'
      ],
      '0058-feel-number-function/0058-feel-number-function-test-01': [
        '004_a', '004_c', '011', '012'
      ],
      '0059-feel-all-function/0059-feel-all-function-test-01': [
        '004', '008', '009', '013', '014', '018'
      ],
      '0060-feel-any-function/0060-feel-any-function-test-01': [
        '004', '008', '009', '014', '018'
      ],
      '0061-feel-median-function/0061-feel-median-function-test-01': [
        '003', '003_a', '004', '005', '012'
      ],
      '0062-feel-mode-function/0062-feel-mode-function-test-01': [
        '003', '003_a', '004', '005', '012'
      ],
      '0063-feel-stddev-function/0063-feel-stddev-function-test-01': [
        '003', '003_a', '004', '005', '007', '008', '008_a', '012'
      ],
      '0066-feel-negation/0066-feel-negation-test-01': [
        '003_b', '003_c', '003_d'
      ],
      '0067-feel-split-function/0067-feel-split-function-test-01': [
        '004', '005', '008', '008_a', '008_b'
      ],
      '0068-feel-equality/0068-feel-equality-test-01': [
        'boolean_008', 'boolean_009', 'context_007', 'date_004', 'datetime_011', 'dt_duration_007',
        'list_016', 'number_007', 'string_005', 'time_012', 'ym_duration_008'
      ],
      '0071-feel-between/0071-feel-between-test-01': [
        'null_001', 'null_002', 'null_003'
      ],
      '0072-feel-in/0072-feel-in-test-01': [
        'null_001'
      ],
      '0080-feel-getvalue-function/0080-feel-getvalue-function-test-01': [
        'decision_001', 'decision_002', 'decision_005', 'decision_006', 'decision_009',
        'decision_010', 'decision_011', 'decision_014'
      ],
      '0081-feel-getentries-function/0081-feel-getentries-function-test-01': [
        'decision_001', 'decision_003', 'decision_007', 'decision_008'
      ],
      '0094-feel-product-function/0094-feel-product-function-test-01': [
        '002', '003', '003_a', '004', '005', '012'
      ],
      '0095-feel-day-of-year-function/0095-feel-day-of-year-function-test-01': [
        'null_001', 'null_002', 'null_003', 'null_004', 'null_005', 'null_007'
      ],
      '0096-feel-day-of-week-function/0096-feel-day-of-week-function-test-01': [
        'null_001', 'null_002', 'null_003', 'null_004', 'null_005', 'null_007'
      ],
      '0097-feel-month-of-year-function/0097-feel-month-of-year-function-test-01': [
        'null_001', 'null_002', 'null_003', 'null_004', 'null_005', 'null_007'
      ],
      '0098-feel-week-of-year-function/0098-feel-week-of-year-function-test-01': [
        'null_001', 'null_002', 'null_003', 'null_004', 'null_005', 'null_007'
      ],
      '0100-arithmetic/0100-arithmetic-test-01': [
        'divide_lhs_number_by_rhs_number_002'
      ],
      '1101-feel-floor-function/1101-feel-floor-function-test-01': [
        '006', '007', '010', '014', '015', '016', '017'
      ],
      '1102-feel-ceiling-function/1102-feel-ceiling-function-test-01': [
        '006', '007', '010', '014', '015', '016', '017'
      ],
      '1110-feel-contains-function/1110-feel-contains-function-test-01': [
        '001_2a4d7448c6', '002_d2a1831b5c', '003_df56e0a1ad'
      ],
      '1111-feel-matches-function/1111-feel-matches-function-test-01': [
        'K-MatchesFunc-1', 'K-MatchesFunc-2', 'K-MatchesFunc-3', 'fn-null-input', 'fn-null-pattern'
      ],
      '1120-feel-duration-function/1120-feel-duration-function-test-01': [
        '001_f2c6cd6866', '002_ddca5756ca', '042_59a0000245', '044_f3b338d877'
      ],
      '1121-feel-years-and-months-duration-function/1121-feel-years-and-months-duration-function-test-01': [
        '001_b24a0c91f2', '002_4e7651ae0e', '003_0886738d31', '004_1bdfef922b', '005_d0a077da4e',
        '006_f20de28d3f', '007_0921c3d61a', '028_77600e7b35', '029_15a0d0d9c1', '030_ec16878596'
      ],
      '1140-feel-string-join-function/1140-feel-string-join-function-test-01': [
        '008', '012_a', '012_b'
      ],
      '1141-feel-round-up-function/1141-feel-round-up-function-test-01': [
        '005', '007', '008', '011_a', '013', '014'
      ],
      '1142-feel-round-down-function/1142-feel-round-down-function-test-01': [
        '005', '007', '008', '011_a', '013', '014'
      ],
      '1143-feel-round-half-up-function/1143-feel-round-half-up-function-test-01': [
        '005', '007', '008', '011_a', '013', '014'
      ],
      '1144-feel-round-half-down-function/1144-feel-round-half-down-function-test-01': [
        '005', '007', '008', '011_a', '013', '014'
      ],
      '1145-feel-context-function/1145-feel-context-function-test-01': [
        '011', '012', '013', '016'
      ],
      '1146-feel-context-put-function/1146-feel-context-put-function-test-01': [
        '005', '006', '008', '009', '012', 'nested008'
      ],
      '1147-feel-context-merge-function/1147-feel-context-merge-function-test-01': [
        '005', '006', '007', '010', '011'
      ],
      '1155-list-replace-function/1155-list-replace-function-test-01': [
        '006', '014', '015'
      ]
    }
  },
  {
    reason: 'feelin computes temporal and duration arithmetic differently from DMN 1.5',
    links: [ 'https://github.com/barmac/feelin/pull/13' ],
    tests: {
      '0099-arithmetic-negation/0099-arithmetic-negation-test-01': [
        'decision_003', 'decision_003_a', 'decision_004', 'decision_004_a'
      ],
      '0100-arithmetic/0100-arithmetic-test-01': [
        'add_lhs_date_to_rhs_dtDuration_002', 'add_lhs_dtDuration_to_rhs_date_002',
        'divide_lhs_dtDuration_by_rhs_dtDuration_001', 'divide_lhs_dtDuration_by_rhs_number_001',
        'divide_lhs_dtDuration_by_rhs_number_003', 'divide_lhs_ymDuration_by_rhs_number_001',
        'divide_lhs_ymDuration_by_rhs_number_003', 'divide_lhs_ymDuration_by_rhs_number_004',
        'divide_lhs_ymDuration_by_rhs_ymDuration_001', 'divide_lhs_ymDuration_by_rhs_ymDuration_002',
        'error_when_add_lhs_dateAndTime_to_rhs_date',
        'error_when_add_lhs_dateAndTime_to_rhs_dateAndTime',
        'error_when_add_lhs_dateAndTime_to_rhs_time', 'error_when_add_lhs_date_to_rhs_date',
        'error_when_add_lhs_date_to_rhs_dateAndTime', 'error_when_add_lhs_date_to_rhs_time',
        'error_when_add_lhs_time_to_rhs_date', 'error_when_add_lhs_time_to_rhs_dateAndTime',
        'error_when_add_lhs_time_to_rhs_time', 'error_when_add_lhs_time_to_rhs_ymDuration',
        'error_when_add_lhs_ymDuration_to_rhs_time', 'error_when_divide_lhs_dateAndTime_by_rhs_date',
        'error_when_divide_lhs_dateAndTime_by_rhs_dateAndTime',
        'error_when_divide_lhs_dateAndTime_by_rhs_dtDuration',
        'error_when_divide_lhs_dateAndTime_by_rhs_time',
        'error_when_divide_lhs_dateAndTime_by_rhs_ymDuration',
        'error_when_divide_lhs_date_by_rhs_date', 'error_when_divide_lhs_date_by_rhs_dateAndTime',
        'error_when_divide_lhs_date_by_rhs_dtDuration', 'error_when_divide_lhs_date_by_rhs_time',
        'error_when_divide_lhs_date_by_rhs_ymDuration',
        'error_when_divide_lhs_dtDuration_by_rhs_date',
        'error_when_divide_lhs_dtDuration_by_rhs_dateAndTime',
        'error_when_divide_lhs_dtDuration_by_rhs_time',
        'error_when_divide_lhs_dtDuration_by_rhs_ymDuration',
        'error_when_divide_lhs_time_by_rhs_date', 'error_when_divide_lhs_time_by_rhs_dateAndTime',
        'error_when_divide_lhs_time_by_rhs_dtDuration', 'error_when_divide_lhs_time_by_rhs_time',
        'error_when_divide_lhs_time_by_rhs_ymDuration',
        'error_when_divide_lhs_ymDuration_by_rhs_date',
        'error_when_divide_lhs_ymDuration_by_rhs_dateAndTime',
        'error_when_divide_lhs_ymDuration_by_rhs_dtDuration',
        'error_when_divide_lhs_ymDuration_by_rhs_time',
        'error_when_exponent_lhs_dateAndTime_exp_rhs_date',
        'error_when_exponent_lhs_dateAndTime_exp_rhs_dateAndTime',
        'error_when_exponent_lhs_dateAndTime_exp_rhs_dtDuration',
        'error_when_exponent_lhs_dateAndTime_exp_rhs_time',
        'error_when_exponent_lhs_dateAndTime_exp_rhs_ymDuration',
        'error_when_exponent_lhs_date_exp_rhs_date',
        'error_when_exponent_lhs_date_exp_rhs_dateAndTime',
        'error_when_exponent_lhs_date_exp_rhs_dtDuration',
        'error_when_exponent_lhs_date_exp_rhs_time',
        'error_when_exponent_lhs_date_exp_rhs_ymDuration',
        'error_when_exponent_lhs_dtDuration_exp_rhs_date',
        'error_when_exponent_lhs_dtDuration_exp_rhs_dateAndTime',
        'error_when_exponent_lhs_dtDuration_exp_rhs_dtDuration',
        'error_when_exponent_lhs_dtDuration_exp_rhs_time',
        'error_when_exponent_lhs_dtDuration_exp_rhs_ymDuration',
        'error_when_exponent_lhs_time_exp_rhs_date',
        'error_when_exponent_lhs_time_exp_rhs_dateAndTime',
        'error_when_exponent_lhs_time_exp_rhs_dtDuration',
        'error_when_exponent_lhs_time_exp_rhs_time',
        'error_when_exponent_lhs_time_exp_rhs_ymDuration',
        'error_when_exponent_lhs_ymDuration_exp_rhs_date',
        'error_when_exponent_lhs_ymDuration_exp_rhs_dateAndTime',
        'error_when_exponent_lhs_ymDuration_exp_rhs_dtDuration',
        'error_when_exponent_lhs_ymDuration_exp_rhs_time',
        'error_when_exponent_lhs_ymDuration_exp_rhs_ymDuration',
        'error_when_multiply_lhs_dateAndTime_by_rhs_date',
        'error_when_multiply_lhs_dateAndTime_by_rhs_dateAndTime',
        'error_when_multiply_lhs_dateAndTime_by_rhs_dtDuration',
        'error_when_multiply_lhs_dateAndTime_by_rhs_time',
        'error_when_multiply_lhs_dateAndTime_by_rhs_ymDuration',
        'error_when_multiply_lhs_date_by_rhs_date',
        'error_when_multiply_lhs_date_by_rhs_dateAndTime',
        'error_when_multiply_lhs_date_by_rhs_dtDuration', 'error_when_multiply_lhs_date_by_rhs_time',
        'error_when_multiply_lhs_date_by_rhs_ymDuration',
        'error_when_multiply_lhs_dtDuration_by_rhs_date',
        'error_when_multiply_lhs_dtDuration_by_rhs_dateAndTime',
        'error_when_multiply_lhs_dtDuration_by_rhs_dtDuration',
        'error_when_multiply_lhs_dtDuration_by_rhs_time',
        'error_when_multiply_lhs_dtDuration_by_rhs_ymDuration',
        'error_when_multiply_lhs_time_by_rhs_date',
        'error_when_multiply_lhs_time_by_rhs_dateAndTime',
        'error_when_multiply_lhs_time_by_rhs_dtDuration', 'error_when_multiply_lhs_time_by_rhs_time',
        'error_when_multiply_lhs_time_by_rhs_ymDuration',
        'error_when_multiply_lhs_ymDuration_by_rhs_date',
        'error_when_multiply_lhs_ymDuration_by_rhs_dateAndTime',
        'error_when_multiply_lhs_ymDuration_by_rhs_dtDuration',
        'error_when_multiply_lhs_ymDuration_by_rhs_time',
        'error_when_multiply_lhs_ymDuration_by_rhs_ymDuration',
        'error_when_subtract_lhs_dateAndTime_minus_rhs_time',
        'error_when_subtract_lhs_date_minus_rhs_time',
        'error_when_subtract_lhs_dtDuration_minus_rhs_date',
        'error_when_subtract_lhs_dtDuration_minus_rhs_dateAndTime',
        'error_when_subtract_lhs_dtDuration_minus_rhs_time',
        'error_when_subtract_lhs_time_minus_rhs_date',
        'error_when_subtract_lhs_time_minus_rhs_dateAndTime',
        'error_when_subtract_lhs_time_minus_rhs_ymDuration',
        'error_when_subtract_lhs_ymDuration_minus_rhs_date',
        'error_when_subtract_lhs_ymDuration_minus_rhs_dateAndTime',
        'error_when_subtract_lhs_ymDuration_minus_rhs_time',
        'multiply_lhs_dtDuration_by_rhs_number_001', 'multiply_lhs_dtDuration_by_rhs_number_002',
        'multiply_lhs_dtDuration_by_rhs_number_003', 'multiply_lhs_dtDuration_by_rhs_number_004',
        'multiply_lhs_dtDuration_by_rhs_number_005', 'multiply_lhs_dtDuration_by_rhs_number_006',
        'multiply_lhs_dtDuration_by_rhs_number_007', 'multiply_lhs_dtDuration_by_rhs_number_008',
        'multiply_lhs_number_by_rhs_dtDuration_001', 'multiply_lhs_number_by_rhs_dtDuration_002',
        'multiply_lhs_number_by_rhs_dtDuration_003', 'multiply_lhs_number_by_rhs_dtDuration_004',
        'multiply_lhs_number_by_rhs_dtDuration_005', 'multiply_lhs_number_by_rhs_dtDuration_006',
        'multiply_lhs_number_by_rhs_dtDuration_007', 'multiply_lhs_number_by_rhs_dtDuration_008',
        'multiply_lhs_number_by_rhs_ymDuration_001', 'multiply_lhs_number_by_rhs_ymDuration_002',
        'multiply_lhs_number_by_rhs_ymDuration_003', 'multiply_lhs_number_by_rhs_ymDuration_004',
        'multiply_lhs_number_by_rhs_ymDuration_005', 'multiply_lhs_number_by_rhs_ymDuration_006',
        'multiply_lhs_number_by_rhs_ymDuration_007', 'multiply_lhs_number_by_rhs_ymDuration_008',
        'multiply_lhs_number_by_rhs_ymDuration_009', 'multiply_lhs_number_by_rhs_ymDuration_010',
        'multiply_lhs_ymDuration_by_rhs_number_001', 'multiply_lhs_ymDuration_by_rhs_number_002',
        'multiply_lhs_ymDuration_by_rhs_number_003', 'multiply_lhs_ymDuration_by_rhs_number_004',
        'multiply_lhs_ymDuration_by_rhs_number_005', 'multiply_lhs_ymDuration_by_rhs_number_006',
        'multiply_lhs_ymDuration_by_rhs_number_007', 'multiply_lhs_ymDuration_by_rhs_number_008',
        'multiply_lhs_ymDuration_by_rhs_number_009', 'multiply_lhs_ymDuration_by_rhs_number_010',
        'subtract_lhs_dateAndTime_minus_rhs_dateAndTime_002',
        'subtract_lhs_dateAndTime_minus_rhs_dateAndTime_003',
        'subtract_lhs_dateAndTime_minus_rhs_dateAndTime_004',
        'subtract_lhs_dateAndTime_minus_rhs_dateAndTime_005',
        'subtract_lhs_dateAndTime_minus_rhs_dateAndTime_007',
        'subtract_lhs_dateAndTime_minus_rhs_dateAndTime_009',
        'subtract_lhs_dateAndTime_minus_rhs_date_001', 'subtract_lhs_dateAndTime_minus_rhs_date_002',
        'subtract_lhs_dateAndTime_minus_rhs_date_003', 'subtract_lhs_date_minus_rhs_dateAndTime_001',
        'subtract_lhs_date_minus_rhs_dateAndTime_002', 'subtract_lhs_date_minus_rhs_dateAndTime_003',
        'subtract_lhs_date_minus_rhs_dtDuration_002', 'subtract_lhs_date_minus_rhs_dtDuration_003',
        'subtract_lhs_date_minus_rhs_dtDuration_004', 'subtract_lhs_date_minus_rhs_dtDuration_005',
        'subtract_lhs_date_minus_rhs_dtDuration_006', 'subtract_lhs_date_minus_rhs_dtDuration_007',
        'subtract_lhs_date_minus_rhs_dtDuration_009', 'subtract_lhs_date_minus_rhs_dtDuration_010'
      ]
    }
  },
  {
    reason: 'feelin does not support the full FEEL temporal value range and parsing rules',
    links: [ 'https://github.com/barmac/feelin/pull/17' ],
    tests: {
      '0007-date-time/0007-date-time-test-01': [
        '001'
      ],
      '0100-arithmetic/0100-arithmetic-test-01': [
        'add_lhs_dateAndTime_to_rhs_dtDuration_009', 'add_lhs_dateAndTime_to_rhs_dtDuration_010',
        'add_lhs_dateAndTime_to_rhs_dtDuration_014', 'add_lhs_dateAndTime_to_rhs_dtDuration_015',
        'add_lhs_dateAndTime_to_rhs_dtDuration_016', 'add_lhs_dateAndTime_to_rhs_dtDuration_017',
        'add_lhs_dateAndTime_to_rhs_dtDuration_018', 'add_lhs_dateAndTime_to_rhs_dtDuration_019',
        'add_lhs_dateAndTime_to_rhs_dtDuration_020', 'add_lhs_dateAndTime_to_rhs_dtDuration_021',
        'add_lhs_dateAndTime_to_rhs_ymDuration_010', 'add_lhs_dateAndTime_to_rhs_ymDuration_011',
        'add_lhs_dateAndTime_to_rhs_ymDuration_012', 'add_lhs_dateAndTime_to_rhs_ymDuration_013',
        'add_lhs_dateAndTime_to_rhs_ymDuration_014', 'add_lhs_dateAndTime_to_rhs_ymDuration_015',
        'add_lhs_dateAndTime_to_rhs_ymDuration_016', 'add_lhs_dateAndTime_to_rhs_ymDuration_017',
        'add_lhs_dtDuration_to_rhs_dateAndTime_009', 'add_lhs_dtDuration_to_rhs_dateAndTime_010',
        'add_lhs_dtDuration_to_rhs_dateAndTime_014', 'add_lhs_dtDuration_to_rhs_dateAndTime_015',
        'add_lhs_dtDuration_to_rhs_dateAndTime_016', 'add_lhs_dtDuration_to_rhs_dateAndTime_017',
        'add_lhs_dtDuration_to_rhs_dateAndTime_018', 'add_lhs_dtDuration_to_rhs_dateAndTime_019',
        'add_lhs_dtDuration_to_rhs_dateAndTime_020', 'add_lhs_dtDuration_to_rhs_dateAndTime_021',
        'add_lhs_ymDuration_to_rhs_dateAndTime_010', 'add_lhs_ymDuration_to_rhs_dateAndTime_011',
        'add_lhs_ymDuration_to_rhs_dateAndTime_012', 'add_lhs_ymDuration_to_rhs_dateAndTime_013',
        'add_lhs_ymDuration_to_rhs_dateAndTime_014', 'add_lhs_ymDuration_to_rhs_dateAndTime_015',
        'add_lhs_ymDuration_to_rhs_dateAndTime_016', 'add_lhs_ymDuration_to_rhs_dateAndTime_017',
        'subtract_lhs_dateAndTime_minus_rhs_dtDuration_005',
        'subtract_lhs_dateAndTime_minus_rhs_dtDuration_006',
        'subtract_lhs_dateAndTime_minus_rhs_dtDuration_008',
        'subtract_lhs_dateAndTime_minus_rhs_dtDuration_009',
        'subtract_lhs_dateAndTime_minus_rhs_dtDuration_010',
        'subtract_lhs_dateAndTime_minus_rhs_dtDuration_011',
        'subtract_lhs_dateAndTime_minus_rhs_dtDuration_012',
        'subtract_lhs_dateAndTime_minus_rhs_dtDuration_013',
        'subtract_lhs_dateAndTime_minus_rhs_dtDuration_014',
        'subtract_lhs_dateAndTime_minus_rhs_dtDuration_015',
        'subtract_lhs_dateAndTime_minus_rhs_dtDuration_016',
        'subtract_lhs_dateAndTime_minus_rhs_dtDuration_017',
        'subtract_lhs_dateAndTime_minus_rhs_ymDuration_009',
        'subtract_lhs_dateAndTime_minus_rhs_ymDuration_010',
        'subtract_lhs_dateAndTime_minus_rhs_ymDuration_011',
        'subtract_lhs_dateAndTime_minus_rhs_ymDuration_012',
        'subtract_lhs_dateAndTime_minus_rhs_ymDuration_013',
        'subtract_lhs_dateAndTime_minus_rhs_ymDuration_014',
        'subtract_lhs_dateAndTime_minus_rhs_ymDuration_015',
        'subtract_lhs_dateAndTime_minus_rhs_ymDuration_016'
      ],
      '1115-feel-date-function/1115-feel-date-function-test-01': [
        '013_d7e901ee86', '014_fad7e00633', '015_1dd66594cf', '016_31f3fef4a0', '027_987c5be372',
        '028_35ca79a6cd', '029_88f5c7c90f', '030_9184a7bfc3'
      ],
      '1116-feel-time-function/1116-feel-time-function-test-01': [
        '067_8285edad7b', '068_ad528abb23', '071_cf9417648b'
      ],
      '1117-feel-date-and-time-function/1117-feel-date-and-time-function-test-01': [
        '010_59f5b47012', '011_eec2d5bdcd', '012_225a105eef', '022_b8b20f0328', '027_ae365197dd',
        '028_1c3d56275f', '060_890c302575', '078_e113dabcdd', '079_2e6f80eb94', '083_4f41731f2a'
      ],
      '1120-feel-duration-function/1120-feel-duration-function-test-01': [
        '011_2169615b94', '034_aa9cbb21a6', '035_93eef01ae7', '036_5f2775875e', '037_8c9ea9c0e6'
      ],
      '1121-feel-years-and-months-duration-function/1121-feel-years-and-months-duration-function-test-01': [
        '017_86744b9a54', '018_8a9ed1d66d', '019_90c2084588'
      ]
    }
  },
  {
    reason: 'feelin does not implement the `range(from)` function',
    links: [ 'https://github.com/barmac/feelin/pull/9' ],
    tests: {
      '1156-range-function/1156-range-function-test-01': [
        '001', '002', '003_a', '003_b', '003_c', '003_d', '003_e', '005_a', '005_b', '005_c',
        '005_d', '005_e', '005_f', '006', '007_a', '008_a', '009_a', '010_a', '011'
      ]
    }
  },
  {
    reason: 'the engine does not execute external (Java) functions',
    tests: {
      '0076-feel-external-java/0076-feel-external-java-test-01': [
        'boxed_001', 'literal_001', 'literal_002', 'literal_003', 'literal_004', 'literal_005',
        'literal_006', 'literal_007', 'literal_008', 'literal_009', 'literal_010', 'literal_011',
        'literal_012', 'varargs_001'
      ]
    }
  },
  {
    reason: 'feelin does not support XML Schema regular expressions',
    links: [ 'https://github.com/barmac/feelin/pull/11' ],
    tests: {
      '1109-feel-replace-function/1109-feel-replace-function-test-01': [
        '023_5c337d3725', '028_96e8c698af'
      ],
      '1111-feel-matches-function/1111-feel-matches-function-test-01': [
        'K2-MatchesFunc-1', 'K2-MatchesFunc-13', 'K2-MatchesFunc-14', 'K2-MatchesFunc-2',
        'K2-MatchesFunc-3', 'K2-MatchesFunc-4', 'K2-MatchesFunc-5', 'K2-MatchesFunc-6',
        'caselessmatch08', 'caselessmatch09', 'caselessmatch10', 'caselessmatch11'
      ]
    }
  },
  {
    reason: 'feelin treats range equality, `null` bounds and iteration differently from DMN 1.5',
    links: [ 'https://github.com/barmac/feelin/pull/14' ],
    tests: {
      '0068-feel-equality/0068-feel-equality-test-01': [
        'range_006', 'range_007', 'range_008', 'range_009', 'range_010', 'range_011', 'range_012'
      ],
      '0072-feel-in/0072-feel-in-test-01': [
        'null_001_a', 'null_001_b'
      ],
      '0074-feel-properties/0074-feel-properties-test-01': [
        'range_011'
      ],
      '0084-feel-for-loops/0084-feel-for-loops-test-01': [
        'decision_017', 'decision_018'
      ]
    }
  },
  {
    reason: 'feelin does not validate rounding scales or round numbers as written',
    links: [ 'https://github.com/barmac/feelin/pull/7' ],
    tests: {
      '1100-feel-decimal-function/1100-feel-decimal-function-test-01': [
        '002_f4ed9cd487'
      ],
      '1141-feel-round-up-function/1141-feel-round-up-function-test-01': [
        '016_b', '017_b'
      ],
      '1142-feel-round-down-function/1142-feel-round-down-function-test-01': [
        '016_b', '017_b'
      ],
      '1143-feel-round-half-up-function/1143-feel-round-half-up-function-test-01': [
        '016_b', '017_b'
      ],
      '1144-feel-round-half-down-function/1144-feel-round-half-down-function-test-01': [
        '016_b', '017_b'
      ]
    }
  },
  {
    reason: 'feelin implements range functions differently from DMN 1.5 Table 78',
    links: [ 'https://github.com/barmac/feelin/pull/10' ],
    tests: {
      '1130-feel-interval/1130-feel-interval-test-01': [
        'coincides', 'during', 'finished by', 'finishes', 'overlaps after', 'overlaps before',
        'started by', 'starts'
      ]
    }
  },
  {
    reason: 'feelin does not parse the `range<T>` type of `instance of`',
    tests: {
      '1156-range-function/1156-range-function-test-01': [
        '001_a', '001_b', '001_c', '001_d', '001_e', '001_g', '001_h', '001_i'
      ]
    }
  },
  {
    reason: 'feelin implements `list replace` differently from DMN 1.5',
    links: [ 'https://github.com/barmac/feelin/pull/4' ],
    tests: {
      '1155-list-replace-function/1155-list-replace-function-test-01': [
        '002', '004', '005', '011_a', '017', '018', '019'
      ]
    }
  },
  {
    reason: 'feelin yields `null` without reporting an error for properties a duration does not have, such as `duration("P1Y").days`',
    tests: {
      '0074-feel-properties/0074-feel-properties-test-01': [
        'dt_duration_001', 'dt_duration_002', 'ym_duration_003', 'ym_duration_004',
        'ym_duration_005', 'ym_duration_006'
      ]
    }
  },
  {
    reason: 'feelin treats range equality, `null` bounds and iteration differently from DMN 1.5, and then yields `null` without reporting an error',
    links: [ 'https://github.com/barmac/feelin/pull/14', 'https://github.com/barmac/feelin/pull/12' ],
    tests: {
      '0072-feel-in/0072-feel-in-test-01': [
        'null_001_c', 'null_001_d'
      ],
      '0084-feel-for-loops/0084-feel-for-loops-test-01': [
        'decision_019', 'decision_019_a', 'decision_025'
      ]
    }
  },
  {
    reason: 'feelin yields `null` without reporting an error for `floor` / `ceiling` with a `null` scale',
    tests: {
      '1101-feel-floor-function/1101-feel-floor-function-test-01': [
        '008', '009'
      ],
      '1102-feel-ceiling-function/1102-feel-ceiling-function-test-01': [
        '008', '009'
      ]
    }
  },
  {
    reason: 'feelin does not treat a non-list filter target as a singleton list',
    links: [ 'https://github.com/barmac/feelin/pull/3' ],
    tests: {
      '0069-feel-list/0069-feel-list-test-01': [
        '012', '014', '016'
      ]
    }
  },
  {
    reason: 'feelin does not compare boolean unary test values with a boolean input',
    links: [ 'https://github.com/barmac/feelin/pull/6' ],
    tests: {
      '0036-dt-variable-input/0036-dt-variable-input-test-01': [
        '_20445ee0-744d-4488-a90e-136701510871', '_58109f16-2e9b-496e-bfff-5f0adbfa0279'
      ]
    }
  },
  {
    reason: 'feelin computes temporal and duration arithmetic differently from DMN 1.5, and then yields `null` without reporting an error',
    links: [ 'https://github.com/barmac/feelin/pull/13', 'https://github.com/barmac/feelin/pull/12' ],
    tests: {
      '0068-feel-equality/0068-feel-equality-test-01': [
        'ym_duration_006'
      ],
      '0100-arithmetic/0100-arithmetic-test-01': [
        'divide_lhs_dtDuration_by_rhs_dtDuration_002'
      ]
    }
  },
  {
    reason: 'feelin accepts context literals with duplicate keys',
    links: [ 'https://github.com/barmac/feelin/pull/8' ],
    tests: {
      '0057-feel-context/0057-feel-context-test-01': [
        '008'
      ]
    }
  },
  {
    reason: 'feelin ignores parameter types of function definitions',
    links: [ 'https://github.com/nikku/feelin/pull/184' ],
    tests: {
      '0082-feel-coercion/0082-feel-coercion-test-01': [
        'fd_002'
      ]
    }
  },
  {
    reason: 'feelin treats durations of different kinds alike in `is`',
    links: [ 'https://github.com/barmac/feelin/pull/2', 'https://github.com/barmac/feelin/pull/13' ],
    tests: {
      '0103-feel-is-function/0103-feel-is-function-test-01': [
        'zero_duration_001'
      ]
    }
  }
];

const REASONS = new Map(
  KNOWN_FAILURES.flatMap(({ reason, links = [], tests }) => {
    const description = [ reason, ...links ].join(', ');

    return Object.entries(tests).flatMap(
      ([ file, ids ]) => ids.map(id => [ `${ file }/${ id }`, description ])
    );
  })
);

/**
 * @param {string} folder
 * @param {string} file
 * @param {string} id
 *
 * @return {string|null} reason
 */
export function getKnownFailure(folder, file, id) {
  return REASONS.get(`${ folder }/${ file }/${ id }`) || null;
}
