"""Async tests for benchmarking."""
import pytest
import asyncio


@pytest.mark.asyncio
async def test_async_simple():
    """Simple async test."""
    await asyncio.sleep(0.001)
    assert True


@pytest.mark.asyncio
async def test_async_gather():
    """Test async gather."""
    async def task(n):
        await asyncio.sleep(0.001)
        return n * 2

    results = await asyncio.gather(task(1), task(2), task(3))
    assert results == [2, 4, 6]


@pytest.mark.asyncio
async def test_async_create_task():
    """Test async create_task."""
    async def worker():
        await asyncio.sleep(0.001)
        return 42

    task = asyncio.create_task(worker())
    result = await task
    assert result == 42


@pytest.mark.asyncio
async def test_async_wait():
    """Test async wait."""
    async def task(n):
        await asyncio.sleep(0.001)
        return n

    tasks = [asyncio.create_task(task(i)) for i in range(3)]
    done, _ = await asyncio.wait(tasks)
    results = [t.result() for t in done]
    assert sorted(results) == [0, 1, 2]


@pytest.mark.asyncio
async def test_async_timeout():
    """Test async with timeout."""
    async def fast_task():
        await asyncio.sleep(0.001)
        return "done"

    result = await asyncio.wait_for(fast_task(), timeout=1.0)
    assert result == "done"
