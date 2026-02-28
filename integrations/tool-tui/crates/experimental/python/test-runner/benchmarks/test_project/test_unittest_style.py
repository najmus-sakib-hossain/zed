"""Unittest-style tests for benchmarking."""
import unittest


class TestBasicAssertions(unittest.TestCase):
    """Test basic assertions."""

    def test_equal(self):
        self.assertEqual(1 + 1, 2)

    def test_not_equal(self):
        self.assertNotEqual(1, 2)

    def test_true(self):
        self.assertTrue(True)

    def test_false(self):
        self.assertFalse(False)

    def test_is(self):
        a = [1, 2, 3]
        b = a
        self.assertIs(a, b)

    def test_is_not(self):
        a = [1, 2, 3]
        b = [1, 2, 3]
        self.assertIsNot(a, b)

    def test_is_none(self):
        self.assertIsNone(None)

    def test_is_not_none(self):
        self.assertIsNotNone(42)


class TestContainerAssertions(unittest.TestCase):
    """Test container assertions."""

    def test_in(self):
        self.assertIn(2, [1, 2, 3])

    def test_not_in(self):
        self.assertNotIn(4, [1, 2, 3])

    def test_count_equal(self):
        self.assertCountEqual([1, 2, 3], [3, 2, 1])

    def test_sequence_equal(self):
        self.assertSequenceEqual([1, 2, 3], [1, 2, 3])

    def test_list_equal(self):
        self.assertListEqual([1, 2], [1, 2])

    def test_tuple_equal(self):
        self.assertTupleEqual((1, 2), (1, 2))

    def test_set_equal(self):
        self.assertSetEqual({1, 2, 3}, {3, 2, 1})

    def test_dict_equal(self):
        self.assertDictEqual({"a": 1}, {"a": 1})


class TestNumericAssertions(unittest.TestCase):
    """Test numeric assertions."""

    def test_greater(self):
        self.assertGreater(5, 3)

    def test_greater_equal(self):
        self.assertGreaterEqual(5, 5)

    def test_less(self):
        self.assertLess(3, 5)

    def test_less_equal(self):
        self.assertLessEqual(5, 5)

    def test_almost_equal(self):
        self.assertAlmostEqual(1.0001, 1.0002, places=3)

    def test_not_almost_equal(self):
        self.assertNotAlmostEqual(1.0, 2.0, places=1)


class TestSetupTeardown(unittest.TestCase):
    """Test with setup and teardown."""

    def setUp(self):
        self.data = [1, 2, 3, 4, 5]

    def tearDown(self):
        self.data = None

    def test_data_length(self):
        self.assertEqual(len(self.data), 5)

    def test_data_sum(self):
        self.assertEqual(sum(self.data), 15)

    def test_data_contains(self):
        self.assertIn(3, self.data)

    def test_data_first(self):
        self.assertEqual(self.data[0], 1)

    def test_data_last(self):
        self.assertEqual(self.data[-1], 5)


if __name__ == "__main__":
    unittest.main()
