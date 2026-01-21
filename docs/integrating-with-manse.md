# Integrating with Manse

Manse is a terminal emulator which exposes a unix socket which, in combination
with its CLI, can be used to emit rich information about what a terminal application is doing.

Right now the primary command for this is:

> manse term-desc "Hello world" 

which will set the description of the current terminal to "hello world." 
