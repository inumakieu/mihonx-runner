# MihonX Runner

A rust powered dex parser and interpreter for running MihonX extensions (and original mihon in the future) cross platform, with Kotlin Multiplatform.

## How it works

There are two phases: Parsing and Interpreting.
The parser creates an Intermediate Representation (IR) of the data inside the dex file, and the Interpreter executes the instructions at runtime.

### Interpreter

The interpreter currently has the following execution loop:

On initialization it loads the classes stored on disk into a DexClass Vector, that holds all the fields, methods, and general information of a class.
It then tries to find the method that is being called in a "main class", which is any class that is inheriting the Source kotlin class.
Once found, it creates a frame (which holds registers, temp value, program counter, and simple class information) and pushes that onto the interpreters frames stack
Anytime a new method is called, it creates a new frame for that method, and pushes it onto the frames stack.

Each instruction is executed one at a time, with a predeterment register count. Register 1 (v1) is ALWAYS "this", refering to the current class. It may be overwritten by an instruction, but at the start of the frames execution, it always needs to be set to v1. 