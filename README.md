This program is to be only used for archival purposes, you should not use this program to download articles and other data from mediawiki instances if the license of the mediawiki instance doesn't allow for such actions. 

# How to use
``ripwik --root <ROOT> --starting-page: <STARTING_PAGE>``
root is the root from which you will download all most of the files. For example ``https://en.wikipedia.org``
starting page is the suffix that comes after the URL of the root. For example ``/wiki/Main_Page`` 

# Downloading and compiling
Download the source code and unpack it in a directory, 
You will need the rust cargo toolchain, 
type in the terminal ``cargo build --release`` and wait for it to finish, the built binary will be nested in one of /target/ subdirectories 
this binary is statically linked, you can copy it to your ``/usr/bin`` or wherever you wish really.
