# Sync point

A simple web application that allow two parties to synchronize given a unique ID.

The default timeout is set to 10 seconds.

## Execution

We first need to start the server in a terminal:

```bash
cargo run
```

And try to open 2 or more terminals to execute the following command:

```bash
# terminal 1
curl -X POST localhost:8080/wait-for-second-party/1

# terminal 2
curl -X POST localhost:8080/wait-for-second-party/1
```

We can also try timeout:
```bash
# no other party will be found after 10 sec
curl -X POST localhost:8080/wait-for-second-party/2
```