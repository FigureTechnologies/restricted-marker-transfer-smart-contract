# Restricted Marker Transfer Smart Contact

This contract facilitates the transfer of restricted coin between addresses.

## Status

[![Latest Release][release-badge]][release-latest]
[![Apache 2.0 License][license-badge]][license-url]
[![Code Coverage][codecov-badge]][codecov-report]

[license-badge]: https://img.shields.io/badge/License-Apache_2.0-blue.svg
[license-url]: https://github.com/FigureTechnologies/restricted-marker-transfer-smart-contract/blob/main/LICENSE
[release-badge]: https://img.shields.io/github/tag/FigureTechnologies/restricted-marker-transfer-smart-contract.svg
[release-latest]: https://github.com/FigureTechnologies/restricted-marker-transfer-smart-contract/releases/latest
[codecov-badge]: https://codecov.io/gh/FigureTechnologies/restricted-marker-transfer-smart-contract/branch/main/graph/badge.svg
[codecov-report]: https://codecov.io/gh/FigureTechnologies/restricted-marker-transfer-smart-contract

## Background

As a holder of a restricted marker, there is no way to transfer those coins without being granted marker transfer permissions or having an account with the permissions to initiate the transfer. This contract allows an account that holds a restricted marker to initiate a transfer that can then be approved or rejected by a marker admin.

## Assumptions

This README assumes you are familiar with writing and deploying smart contracts to the
[provenance](https://docs.provenance.io/) blockchain.
See the `provwasm` [tutorial](https://github.com/provenance-io/provwasm/blob/main/docs/tutorial/01-overview.md)
for details.

### [Provenance Testnet](https://github.com/provenance-io/testnet) Deployments
#### [pio-testnet-1](https://github.com/provenance-io/testnet/tree/main/pio-testnet-1)

| Contract Version | Code ID |
|------------------|---------|
| 0.1.0            | 157     |
| 0.1.1            | 166     |

## Blockchain Quickstart

Checkout provenance v1.16.0, install the `provenanced` command and start a 4-node localnet.

```bash
git clone https://github.com/provenance-io/provenance.git
cd provenance && git checkout v1.16.0
make install
make localnet-start
```

## Accounts

Accounts need to be set up for example users and marker admins.

User 1

```bash
provenanced keys add user1 \
    --home build/node0 --keyring-backend test --testnet --hd-path "44'/1'/0'/0/0" --output json | jq

{
  "name": "user1",
  "type": "local",
  "address": "tp10nnm70y8zc5m8yje5zx5canyqq639j3ph7mj8p",
  "pubkey": "tppub1addwnpepqf4feq9n484c6tvpcugkp0l78mffld8aphq8wqehx53pekcf2l5pkuajggq",
  "mnemonic": "seminar tape camp attract student make hollow pyramid obtain bamboo exit donate dish drip text foil news film assist access pride decline reason lonely"
}
```

User 2

```bash
provenanced keys add user2 \
    --home build/node0 --keyring-backend test --testnet --hd-path "44'/1'/0'/0/0" --output json | jq

{
  "name": "user2",
  "type": "local",
  "address": "tp1m4arun5y9jcwkatq2ey9wuftanm5ptzsg4ppfs",
  "pubkey": "tppub1addwnpepqgw8y7dpx4xmlaun5u55qrq4e05jtul6nu94afq3tvr7e8d4xx6ujzf79jz",
  "mnemonic": "immense ordinary august exclude loyal expire install tongue ski bounce sock buffalo range begin glory inch index float medal kid empty wheel badge find"
}
```

Admin 1

```bash
provenanced keys add admin1 \
    --home build/node0 --keyring-backend test --testnet --hd-path "44'/1'/0'/0/0" --output json | jq

{
  "name": "admin1",
  "type": "local",
  "address": "tp15nauudez3yvrma9mfve7t9hnnnlkgc7fwps85d",
  "pubkey": "{\"@type\":\"/cosmos.crypto.secp256k1.PubKey\",\"key\":\"AlOF+u9+kMmP3mLlny+u2S7WBgDnJqJOwzJVXCFJZOgI\"}",
  "mnemonic": "develop glory absurd glory march valve hunt barely inform luxury ahead miss eye minimum assault meat pair shoot magic develop argue exact believe faint"
}
```

If you want to use the addresses from this document, use the mnemonics above to restore the keys
locally.

For example:

```bash
provenanced keys add user1 --recover \
    --home build/node0 --keyring-backend test --testnet --hd-path "44'/1'/0'/0/0"
```

## Fee Payment

Fund the example accounts with `nhash` to pay network fees.

```bash
provenanced tx bank send \
    $(provenanced keys show -a node0 --home build/node0 --keyring-backend test --testnet) \
    $(provenanced keys show -a user1 --home build/node0 --keyring-backend test --testnet) \
    100000000000nhash \
    --from node0 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto --gas-prices 1905nhash --gas-adjustment 2 \
    --broadcast-mode block \
    --yes \
    --testnet -o json  | jq
```

```bash
provenanced tx bank send \
    $(provenanced keys show -a node0 --home build/node0 --keyring-backend test --testnet) \
    $(provenanced keys show -a user2 --home build/node0 --keyring-backend test --testnet) \
    100000000000nhash \
    --from node0 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto --gas-prices 1905nhash --gas-adjustment 2 \
    --broadcast-mode block \
    --yes \
    --testnet -o json  | jq
```

```bash
provenanced tx bank send \
    $(provenanced keys show -a node0 --home build/node0 --keyring-backend test --testnet) \
    $(provenanced keys show -a admin1 --home build/node0 --keyring-backend test --testnet) \
    100000000000nhash \
    --from node0 \
    --keyring-backend test \
    --home build/node0 \
    --chain-id chain-local \
    --gas auto --gas-prices 1905nhash --gas-adjustment 2 \
    --broadcast-mode block \
    --yes \
    --testnet -o json  | jq
```


## Store the Wasm

Store the optimized smart contract Wasm on-chain. This assumes you've copied `artifacts/restricted_marker_transfer`
to the provenance root dir (ie where the localnet was started from).

```bash
provenanced tx wasm store restricted_marker_transfer.wasm \
  --from admin1 \
  --home build/node0 --keyring-backend test \
  --chain-id chain-local \
  --gas auto --gas-prices 1905nhash --gas-adjustment 1.3 \
  --testnet \
  --yes -o json  | jq
```


## Instantiate the contract

Instantiate the contract using the `code_id` returned from storing the Wasm. Note the contract address returned

```bash
build/provenanced tx wasm instantiate 17 \
  '{"name":"marker-transfer-local1" }' \
  --label restricted-marker-transfer1 \
  --admin $(provenanced keys show -a admin1 --home build/node0 --keyring-backend test --testnet) \
  --from admin1 \
  --home build/node0 --keyring-backend test \
  --chain-id chain-local \
  --gas auto --gas-prices 1905nhash --gas-adjustment 1.3 \
  --testnet \
  --yes
```

```text
logs:
- events:
  - attributes:
    - key: _contract_address
      value: tp15fnweczx7273jc6tmuuacmkl6zk6mq8ffh8r0artxp9srdpctcesek7uac
    - key: code_id
      value: "17"
    type: instantiate
  - attributes:
    - key: action
      value: /cosmwasm.wasm.v1.MsgInstantiateContract
    - key: module
      value: wasm
    - key: sender
      value: tp15nauudez3yvrma9mfve7t9hnnnlkgc7fwps85d
    type: message
```

## Marker creation
Create a restricted marker representing private company stock for a company named `example-co`

```bash
provenanced tx marker new "50000example-co.stock" \
  --type RESTRICTED \
  --from admin1 \
  --home build/node0 --keyring-backend test \
  --chain-id chain-local \
  --gas auto --gas-prices 1905nhash --gas-adjustment 1.3 \
  --testnet \
  --yes
```
Grant marker admin access to `admin1`
```bash
provenanced tx marker grant $(provenanced keys show -a admin1 --home build/node0 --keyring-backend test --testnet) example-co.stock admin,withdraw,burn,mint,transfer \
  --from admin1 \
  --home build/node0 --keyring-backend test \
  --chain-id chain-local \
  --gas auto --gas-prices 1905nhash --gas-adjustment 1.3 \
  --testnet \
  --yes
```
Finalize the marker
```bash
provenanced tx marker finalize example-co.stock \
  --from admin1 \
  --home build/node0 --keyring-backend test \
  --chain-id chain-local \
  --gas auto --gas-prices 1905nhash --gas-adjustment 1.3 \
  --testnet \
  --yes
```
Activate the marker
```bash
provenanced tx marker activate example-co.stock \
  --from admin1 \
  --home build/node0 --keyring-backend test \
  --chain-id chain-local \
  --gas auto --gas-prices 1905nhash --gas-adjustment 1.3 \
  --testnet \
  --yes
```

Grant marker transfer permission to the smart contract, so it can move coin.
```bash
provenanced tx marker grant tp15fnweczx7273jc6tmuuacmkl6zk6mq8ffh8r0artxp9srdpctcesek7uac example-co.stock transfer \
  --from admin1 \
  --home build/node0 --keyring-backend test \
  --chain-id chain-local \
  --gas auto --gas-prices 1905nhash --gas-adjustment 1.3 \
  --testnet \
  --yes
```

Now distribute shares of example-co stock to `user1`
```bash
provenanced tx marker withdraw example-co.stock 1000example-co.stock $(provenanced keys show -a user1 --home build/node0 --keyring-backend test --testnet)  \
  --from admin1 \
  --home build/node0 --keyring-backend test \
  --chain-id chain-local \
  --gas auto --gas-prices 1905nhash --gas-adjustment 1.3 \
  --testnet \
  --yes
```

## Contract execution example
### Transfer
`user1` can now transfer shares of example-co stock to `user2` using the smart contract.

NOTE: you can get the address for `user2` with the following command:

```bash
provenanced keys show -a user2 --home build/node0 -t
```
Transfer 5 shares of `example-co.stock` from user1 to user2:
```bash

# first grant authz permission so the contract can escrow the coin for the transfer
  provenanced tx marker grant-authz \
    tp15fnweczx7273jc6tmuuacmkl6zk6mq8ffh8r0artxp9srdpctcesek7uac \
    "transfer" \
    --transfer-limit 5example-co.stock \
    --from user1 \
    --home build/node0 --keyring-backend test \
    --chain-id chain-local \
    --gas auto --gas-prices 1905nhash --gas-adjustment 1.3 \
    --testnet \
    --yes -o json | jq

# initiate transfer using a unique uuid as the id attribute
provenanced tx wasm execute tp15fnweczx7273jc6tmuuacmkl6zk6mq8ffh8r0artxp9srdpctcesek7uac \
    '{"transfer":{"id":"54c4f5d9-5253-43ac-9011-bbc52465581e", "denom":"example-co.stock",  "amount":"5", "recipient": "tp1m4arun5y9jcwkatq2ey9wuftanm5ptzsg4ppfs"}}' \
    --from user1 \
    --home build/node0 --keyring-backend test \
    --chain-id chain-local \
    --gas auto --gas-prices 1905nhash --gas-adjustment 1.3 \
    --testnet \
    --yes -o json | jq
```

### Query transfers

query all pending transfers
```bash
provenanced q wasm contract-state smart tp15fnweczx7273jc6tmuuacmkl6zk6mq8ffh8r0artxp9srdpctcesek7uac \
    '{"get_all_transfers":{}}' \
    --ascii -o json \
    --chain-id chain-local \
    --testnet | jq
```

### Approve
Now the marker admin can approve the transfer
```bash
provenanced tx wasm execute tp15fnweczx7273jc6tmuuacmkl6zk6mq8ffh8r0artxp9srdpctcesek7uac \
    '{"approve_transfer":{"id":"54c4f5d9-5253-43ac-9011-bbc52465581e"}}' \
    --from admin1 \
    --home build/node0 --keyring-backend test \
    --chain-id chain-local \
    --gas auto --gas-prices 1905nhash --gas-adjustment 1.3 \
    --testnet \
    --yes -o json | jq
```
You can check the balance of `user2` to see that the transfer was successful
```bash
provenanced q bank balances $(provenanced keys show -a user2 --home build/node0 --keyring-backend test --testnet) -t
```
## Other contract actions
### Cancel
A user can cancel a transfer before it is approved or rejected:
```bash
provenanced tx wasm execute tp15fnweczx7273jc6tmuuacmkl6zk6mq8ffh8r0artxp9srdpctcesek7uac \
    '{"cancel_transfer":{"id":"54c4f5d9-5253-43ac-9011-bbc52465581e"}}' \
    --from user1 \
    --home build/node0 --keyring-backend test \
    --chain-id chain-local \
    --gas auto --gas-prices 1905nhash --gas-adjustment 1.3 \
    --testnet \
    --yes -o json | jq
```
### Reject
The marker admin can reject a transfer:
```bash
provenanced tx wasm execute tp15fnweczx7273jc6tmuuacmkl6zk6mq8ffh8r0artxp9srdpctcesek7uac \
    '{"reject_transfer":{"id":"54c4f5d9-5253-43ac-9011-bbc52465581e"}}' \
    --from admin1 \
    --home build/node0 --keyring-backend test \
    --chain-id chain-local \
    --gas auto --gas-prices 1905nhash --gas-adjustment 1.3 \
    --testnet \
    --yes -o json | jq
```
