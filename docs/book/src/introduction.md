# Introduction

[`daft`](https://www.getdaft.io/) is a fast and distributed python query engine built on top of the rust programming language.
One of its biggest features is its support over multi-modal data.
As such, many developers are interested in getting their hands on daft and playing around with it.

Downloading and running daft on a single node (most likely your local machine) is simple.
This largely just includes downloading daft in your python project, importing it into your script or notebook, and directly interfacing with it.

Experimenting with daft in a distributed environment, however, becomes quite a bit more challenging.
In order to use distributed daft, developers must use the `ray` cluster management software (the sdk and/or the cli tool).
However, familiarizing oneself with ray and its intricacies is not simple.
Developers are often required to be aware of some arcane knowledge into the intersection of ray and their choice of cloud-provider.
The story can be so challenging that after multiple attempts, the developer may just give up on trying daft in a distributed manner altogether.

## Simplifying launches

Ideally, developers should be able to bring their own cloud and get up and running with running daft in a distributed setting as quickly as possible.
This is where `daft-launcher` comes into the picture.

Daft-launcher is a command-line tool that aims to provide some simpler abstractions over ray, thus enabling a quick uptime during experimentation.
This is also a great tool during actual development; the ability to quickly spin up and manage clusters is a powerful asset to any data engineer.

This book will aim to introduce daft-launcher and how it enables the developers to get up and running quickly with daft.
