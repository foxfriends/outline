# Outline

Generic literate programming transpiler. This project aims to provide a modern, developer friendly
literate programming tool. After a brief search online, other options claiming to be "modern" are
years old, and have some pretty ugly syntax (in my opinion). If I am wrong in this regard, please
open an issue to correct me!

In contrast, Outline works with familiar syntax, which can be further customized to suit your needs
exactly. It uses pluggable, configurable input formats, with out-of-the-box support for:
*   Markdown;
*   Latex;
*   HTML; and
*   (some approximation of) Bird style

See the examples directory for full working examples in each style.

## Features

In all styles, the code sections are handled the same way, supporting:
*   macros;
*   meta variable interpolation;
*   comment extraction; and
*   named entrypoints;
*   multiple languages in one file

### Macros

Macros are what enables the literate program to be written in logical order for the human reader.
Using Outline, this is accomplished by naming the code blocks, and then referencing them later by
"invoking" the macro. While the syntax for naming a code block is specific to the documentation
style, macro invocation is the same.

By default, macro invocations start with a long arrow `==>` and end with a period `.`. Both of these
sequences can be customized to suit your needs better. The only restriction with macro invocations
is that they must be the only thing on the line. That is, this is valid:

```rs
fn main() {
  ==> Calculate the very complex result.
  ==> Print the results for the user.
}
```

But this would not actually invoke any macros:

```rs
fn main() {
  if (==> A very complex condition is true.) {
    ==> Do something cool.
  }
}
```

Another feature of macros to note is that if two code blocks have the same name, they are
concatenated, in the order they are written. This can be very useful in defining global variables or
listing imports closer to the parts where they are used.

### Meta Variables

If you consider a macro invocation like a function call, then meta variables are like parameters.

By default, to indicate that a macro includes a meta variable, the name of the variable must be part
of the name of the macro, delimited by `@{` and `}`.

Then that meta variable may be used within the macro by again using its name within the `@{` and `}`
in the code.

Finally, a macro with meta variables is invoked by replacing the name of the variable with its
value in the invocation.

An example:

```tex
Here is our macro with meta variables:

\begin{code}[language=rs,name={Say @{something} to @{someone}}]
println!("Hey, @{someone}! I was told to tell you \"@{something}\"");
\end{code}

Now, to say things to many people:

\begin{code}[language=rs]
==> Say @{Hello} to @{Jim}.
==> Say @{How are you} to @{Tom}.
==> Say @{I am good!} to @{Angela}.
\end{code}
```

This feature allows for more flexibility when writing macros, as well as possibly making the intent
clearer.

### Extracted comments

By default, the comment extraction sequence is set to `//`, purely for familiarity. Any text after
(and including) this sequence is extracted from the code block, and not rendered to the tangled
source code. Note that, since the comments are removed completely when compiling, they do not have
to use the actual line comment indicator from you programming language. In fact, it may be better to
choose a sequence that is *not* the regular comment indicator so that you can still have comments in
your tangled code output.

Now that these comments are extracted, it is possible to handle them differently in the weaved
documentation file. Though some formats do not support any special behaviour, and simply write these
comments back into the code, some are able to provide special rendering. In particular, the standard
Markdown and HTML styles are able to render extracted comments in `<aside>` tags, which can then be
rendered nicely using CSS.

See the HTML example for an example of one way to render the extracted comments.

### Named Entrypoints

By default, the entrypoint of the program is always the unnamed code block. However, this limits the
output of one input file to always be the same source code. It also means that you can't have a name
on the entrypoint in the documentation, which could be useful.

To get around this, an entrypoint name can be passed to Outline on the command line. Then, instead
of starting at the unnamed code block, it will start at the code block with this name.

Note that if you use a named entrypoint, there is no way to reference the unnamed code blocks as
macros. You can, however, use the unnamed code blocks to provide examples, for example, to the
readers of the documentation, so they are still useful.

### Multiple languages

Some documentation formats allow you to indicate the language that a code block is written in. In
fact, it is recommended that you always include the language when you write a code block,
particularly if multiple programming languages are used within the same document.

By properly labelling all code blocks, it is then possible to write a program in multiple
programming languages at once. Whether this is practical or not remains to be seen, but it is
supported nonetheless. By then supplying a language name on the command line, only code blocks in
that language are used when generating the tangled source. For example, here is a trivial program
written in two languages:

```tex
Here we have hello world in Ruby:

\begin{code}[language=rb]
puts "Hello world"
\end{code}

And here it is again in Rust:

\begin{code}[language=rs]
fn main() {
  println!("Hello world");
}
\end{code}
```

Compiling this with no language supplied with just ignore language information, so a single output
will be generated containing both languages. However, supplying the `--language rb` flag to Outline
will cause only the code blocks tagged with `rb` will be used to generate code.
