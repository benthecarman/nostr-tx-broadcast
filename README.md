# nostr-tx-broadcast

## Introduction

nostr-tx-broadcast is a tool that listens to nostr relays for bitcoin transaction events and broadcasts them to the bitcoin network.

An implementation of https://github.com/nostr-protocol/nips/pull/476

## Overview

nostr-tx-broadcast does the following:

-    Generates a set of Nostr keys.
-    Creates a Nostr client and adds multiple relays.
-    Connects to the relays.
-    Subscribes to Bitcoin transactions (with a custom event kind of 28333) from the relays.
-    Listens for incoming Bitcoin transactions and decodes them.
-    Broadcasts the decoded transactions to the mempool.space API.

## Installation

Start by just doing:

```bash
cargo run -- -r <relay>
```
