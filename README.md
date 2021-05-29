# Blockchain-rs

Simple proof of concept blockchain written in Rust. _Do not build the next Dogecoin on top of this, I won't be held responsible!_

Jokes aside, this was built for learning purposes, it support basic blockchain capabilities such as multi-node Proof-of-Work consensus. It exposes a HTTP API with the following endpoints:

```rust
.service(web::resource("/nodes/register").route(web::post().to(register)))
.service(web::resource("/nodes/resolve").route(web::get().to(resolve)))
.service(web::resource("/mine").route(web::get().to(mine)))
.service(web::resource("/chain").route(web::get().to(chain)))
.service(web::resource("/transactions/new").route(web::post().to(transaction)))
```

There a lot of missing features I would like to add someday for fun:
1. Bootstrap node and node registration broadcasting (currently you have to manually register every node);
2. RPC-based communication;
3. Persistence to disk (currently everything is in-memory);
4. Public-key encryption (right now it's all UUID + sha hashing);
5. Different consensus algorithms such as Proof-of-Stake;
6. Much more validations and assertions;
7. For PoW: configurable and dynamic mining difficulty (right now it's hardcoded);
8. And much more.

## Running nodes and interacting with them

To run a node you can use: `cargo run -- 8080` where the only argument is the port exposed by that single node.

When running multiple nodes, you must register each one of them manually. For instance, to register `8080` to the `8081` node:
```
 curl -X POST -d '{"addresses": ["http://localhost:8080"]}' -H "Content-type: application/json" http://localhost:8081/nodes/register
```

Note that you also have to register `8081` to `8080`. This is necessary for the consensus algorithm to work properly; Once their chain differ, they will query each other's chain and pick the longest one.

To query the current chain in a node you can: 

```
curl http://localhost:8080/chain | json_pp
[
   {
      "index" : 1,
      "previous_hash" : "1",
      "proof" : 100,
      "timestamp" : {
         "nanos_since_epoch" : 658139000,
         "secs_since_epoch" : 1622306099
      },
      "transactions" : []
   },
   {
      "index" : 2,
      "previous_hash" : "ECADC938053B0063C9C5ADCA6EBF64B7E8004FD1FB5B196DD782C59D3470213B",
      "proof" : 248000,
      "timestamp" : {
         "nanos_since_epoch" : 191055800,
         "secs_since_epoch" : 1622306146
      },
      "transactions" : [
         {
            "amount" : 1,
            "recipient" : "9509df17-9979-4b36-9942-e8c4cbf09830",
            "sender" : "0"
         }
      ]
   },
   {
      "index" : 3,
      "previous_hash" : "4F80AFD7194667E0AA77FD8FC745CE36BBC19F7C9CCC930D50CC826C4BD39FEF",
      "proof" : 101720,
      "timestamp" : {
         "nanos_since_epoch" : 304410600,
         "secs_since_epoch" : 1622306169
      },
      "transactions" : [
         {
            "amount" : 1,
            "recipient" : "55098877-a3a4-40ee-b68a-1f7e459aa20d",
            "sender" : "0"
         }
      ]
   },
   {
      "index" : 4,
      "previous_hash" : "ACB813083B3026D5A75E1E0FE53A30FEBC3BCC8BAA41FFDFB2938845F9462763",
      "proof" : 1282,
      "timestamp" : {
         "nanos_since_epoch" : 988313400,
         "secs_since_epoch" : 1622306230
      },
      "transactions" : [
         {
            "amount" : 1,
            "recipient" : "9509df17-9979-4b36-9942-e8c4cbf09830",
            "sender" : "0"
         }
      ]
   }
]
```

Sender "0" means it's an award for mining.

To send a transaction: 
```
curl -X POST -d '{"amount": 10.0, "sender": "349857dhkfjg", "recipient": "234098dflsdf"}' -H "Content-type: application/json" http://localhost:8080/transactions/new
```

The transaction will be added to the next mined block.