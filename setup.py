# credit to https://github.com/samuelcolvin/rtoml/blob/main/setup.py

import re
from pathlib import Path

from setuptools import setup
from setuptools_rust import Binding, RustExtension


THIS_DIR = Path(__file__).resolve().parent

# VERSION is set in Cargo.toml
cargo = Path(THIS_DIR / 'Cargo.toml').read_text()
VERSION = re.search('version *= *"(.*?)"', cargo).group(1)
DESCRIPTION = re.search('version *= *"(.*?)"', cargo).group(1)

try:
    long_description = (THIS_DIR / 'README.md').read_text()
except FileNotFoundError:
    long_description = DESCRIPTION

setup(
    name='fusion_blossom',
    version=VERSION,
    description=DESCRIPTION,
    long_description=long_description,
    long_description_content_type='text/markdown',
    author='Yue Wu',
    author_email='wuyue16pku@gmail.com',
    url='https://github.com/yuewuo/fusion-blossom',
    license='MIT',
    python_requires='>=3.6',
    rust_extensions=[RustExtension('fusion_blossom.fusion_blossom', binding=Binding.PyO3)],
    package_data={'fusion_blossom': ['py.typed']},
    packages=['fusion_blossom'],
    zip_safe=False,
    classifiers=[
        "Programming Language :: Rust",
        'Programming Language :: Python',
        'Programming Language :: Python :: 3',
        'Programming Language :: Python :: 3 :: Only',
        'Programming Language :: Python :: 3.7',
        'Programming Language :: Python :: 3.8',
        'Programming Language :: Python :: 3.9',
        "Programming Language :: Python :: Implementation :: CPython",
        "Programming Language :: Python :: Implementation :: PyPy",
        'Intended Audience :: Developers',
        'Intended Audience :: Information Technology',
        'Intended Audience :: System Administrators',
        'License :: OSI Approved :: MIT License',
        'Operating System :: Unix',
        'Operating System :: POSIX :: Linux',
        'Environment :: Console',
        'Environment :: MacOS X',
        'Topic :: Software Development :: Libraries :: Python Modules',
        'Topic :: Internet',
    ],
)
