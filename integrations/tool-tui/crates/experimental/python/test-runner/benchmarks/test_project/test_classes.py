"""Class-based tests for benchmarking."""


class TestMathOperations:
    """Test class for math operations."""

    def test_add(self):
        assert 2 + 2 == 4

    def test_subtract(self):
        assert 10 - 5 == 5

    def test_multiply(self):
        assert 6 * 7 == 42

    def test_divide(self):
        assert 100 / 10 == 10

    def test_power(self):
        assert 2**10 == 1024

    def test_modulo(self):
        assert 17 % 5 == 2

    def test_floor_division(self):
        assert 17 // 5 == 3

    def test_negative(self):
        assert -(-5) == 5


class TestStringOperations:
    """Test class for string operations."""

    def test_upper(self):
        assert "hello".upper() == "HELLO"

    def test_lower(self):
        assert "WORLD".lower() == "world"

    def test_strip(self):
        assert "  test  ".strip() == "test"

    def test_split(self):
        assert "a,b,c".split(",") == ["a", "b", "c"]

    def test_join(self):
        assert "-".join(["a", "b", "c"]) == "a-b-c"

    def test_replace(self):
        assert "hello".replace("l", "x") == "hexxo"

    def test_startswith(self):
        assert "python".startswith("py")

    def test_endswith(self):
        assert "python".endswith("on")


class TestListOperations:
    """Test class for list operations."""

    def test_append(self):
        lst = [1, 2]
        lst.append(3)
        assert lst == [1, 2, 3]

    def test_extend(self):
        lst = [1, 2]
        lst.extend([3, 4])
        assert lst == [1, 2, 3, 4]

    def test_insert(self):
        lst = [1, 3]
        lst.insert(1, 2)
        assert lst == [1, 2, 3]

    def test_remove(self):
        lst = [1, 2, 3]
        lst.remove(2)
        assert lst == [1, 3]

    def test_pop(self):
        lst = [1, 2, 3]
        val = lst.pop()
        assert val == 3 and lst == [1, 2]

    def test_sort(self):
        lst = [3, 1, 2]
        lst.sort()
        assert lst == [1, 2, 3]

    def test_reverse(self):
        lst = [1, 2, 3]
        lst.reverse()
        assert lst == [3, 2, 1]

    def test_index(self):
        lst = [1, 2, 3]
        assert lst.index(2) == 1
