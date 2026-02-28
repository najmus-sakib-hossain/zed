"""Setup script for dcp-client package."""

from setuptools import setup, find_packages

with open("README.md", "r", encoding="utf-8") as fh:
    long_description = fh.read()

setup(
    name="dcp-client",
    version="0.1.0",
    author="DCP Team",
    author_email="dcp@example.com",
    description="Python SDK for the Development Context Protocol (DCP)",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/example/dcp",
    packages=find_packages(),
    classifiers=[
        "Development Status :: 3 - Alpha",
        "Intended Audience :: Developers",
        "License :: OSI Approved :: MIT License",
        "Operating System :: OS Independent",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
        "Programming Language :: Python :: 3.12",
        "Topic :: Software Development :: Libraries :: Python Modules",
    ],
    python_requires=">=3.8",
    install_requires=[],
    extras_require={
        "sse": ["aiohttp>=3.8.0"],
        "dev": [
            "pytest>=7.0.0",
            "pytest-asyncio>=0.21.0",
            "aiohttp>=3.8.0",
        ],
    },
)
