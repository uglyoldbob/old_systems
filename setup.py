import setuptools

with open("README.md", "r") as fh:
    long_description = fh.read()

from nes.hdl import version_str

setuptools.setup(
    name="pythondata-old-systems",
    version=version_str,
    author="Thomas Epperson",
    author_email="thomas.epperson@gmail.com",
    description="""\
Python module containing hdl files for emulating old consoles.""",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/uglyoldbob/old_systems",
    classifiers=[
        "Programming Language :: Python :: 3",
        "License :: Unknown",
        "Operating System :: OS Independent",
    ],
    python_requires='>=3.5',
    zip_safe=False,
    packages=setuptools.find_packages(),
    package_data={
    	'nes/hdl': ['**'],
    },
    include_package_data=True,
    project_urls={
        "Bug Tracker": "https://github.com/uglyoldbob/old_systems/issues",
        "Source Code": "https://github.com/uglyoldbob/old_systems",
    },
)