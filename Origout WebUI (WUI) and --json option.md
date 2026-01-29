origout has to have a CLI tool that runs a locally hosted Go http server (Go, Fiber, Templ, GORM GEN).

The "og-wui" tool isn't designed to be hosted on the public internet, as it automatically uses the cryptographic keys of the server it is running on, aka programmer's dev machine. 

It has to use JS for convenience, like casting reactions or posting comments without refreshing the page. The site has to be completely functional with JS off, without any exeptions.

The UI has to be familiar to GitHub web UI and Forgejo, but more simplified. Something between SourceHut and GitHub. It also has to have first class support for dark mode, and high contrast themes too. 

The WUI is just a wrapper around the CLI. All origout commands have to accept a `--json` flag which returns a consistent, versioned standard output. This opens room for straightforward process in further 3rd party tooling development as well. 