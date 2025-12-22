from setuptools import setup, find_packages

setup(
    name="nhargrex",
    version="0.1",
    packages=find_packages(),
    py_modules=["sensors_nhargrex_firestore"], 
    install_requires=[
        # List any dependencies your package has here
    ],
    author="Nicholas Hargreaves",
    author_email="nicholas.hargreaves@outlook.com",
    description="Connection to FCM",
    long_description=open("README.md").read(),
    long_description_content_type="text/markdown",
    url="https://github.com/nhargre1/my_package",
    classifiers=[
        "Programming Language :: Python :: 3",
        "License :: OSI Approved :: MIT License",
        "Operating System :: OS Independent",
    ],
    python_requires='>=3.6',
)