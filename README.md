A system for opening paths and URLs (anything accepted by `xdg-open` and the like) in an X11 environment, passing the value through X11 itself rather than by calling a command. This allows URIs on a remote machine to be opened on the user's local machine when X-forwarding.

# Usage

On the local machine, run
```
$ x11uri server
```
Then to open a URI, use
```
$ x11uri client <URI>
```
or the included binary
```
$ x11uriclient <URI>
```
similarly to how you would use `xdg-open`. Regardless of what machine this command is run on, the URI will be opened in the environment running the x11uri server, including (probably) Windows and Mac machines (I have not tested either of these).

To get all programs on the remote machine to use the x11uri system, replace your `xdg-open` or equivalent with the x11uriclient binary, or rename the x11uriclient binary to `xdg-open`, place it in a folder, and add the folder to your path:
```
$ SOMEPATH=/some/path/to/a/folder
$ cp x11uriclient $SOMEPATH/xdg-open
$ export PATH=$SOMEPATH:$PATH
```
To make sure this works for all programs, put these lines in .login or something on the remote machine so that all processes inherit them.