The binary has to come with a quite large and comprehensive list of node indexes.

A node index is a very lightweight JSON http endpoint that points to a list of discovered nodes. The software that generates it is one binary in the origout distribution, specifically "origout-node-index".

It has to be hostable in GitHub and similar platforms too. Has to try reading the last checkup node index, connect to those, and update its list and commit that.

A list of many nodes and node indexes both from me and other devs have to be included in the hardcoded binary, to ensure that initial connectivity isn't dependent to official cache nodes