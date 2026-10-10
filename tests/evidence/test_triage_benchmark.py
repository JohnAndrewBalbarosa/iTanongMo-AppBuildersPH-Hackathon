import importlib.util
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location(
    "benchmark", Path(__file__).with_name("triage-benchmark.py")
)
benchmark = importlib.util.module_from_spec(spec)
spec.loader.exec_module(benchmark)


class ClassificationMetricTests(unittest.TestCase):
    def test_confusion_counts_include_false_positives_and_missed_labels(self):
        cases = [{"labels": ["a"]}, {"labels": ["a", "b"]}, {"labels": []}]
        result = benchmark.classification_metrics(cases, [["a"], [], ["b"]], ["a", "b"])
        self.assertEqual(result["per_label"]["a"]["true_positive"], 1)
        self.assertEqual(result["per_label"]["a"]["false_negative"], 1)
        self.assertEqual(result["per_label"]["b"]["false_positive"], 1)
        self.assertEqual(result["per_label"]["b"]["false_negative"], 1)
        self.assertAlmostEqual(result["macro_f1"], 1 / 3)
        self.assertAlmostEqual(result["exact_match_rate"], 1 / 3)

    def test_mismatched_predictions_fail_instead_of_changing_denominator(self):
        with self.assertRaises(ValueError):
            benchmark.classification_metrics([{"labels": ["a"]}], [], ["a"])


if __name__ == "__main__":
    unittest.main()
