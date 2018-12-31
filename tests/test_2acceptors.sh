#!/usr/bin/env bash

echo "Basic tests for the Atomic Broadcast implementation (using Multi-Paxos) using only 2 acceptors."

STARTERS="$1"
CONFIG=`pwd`/../Config.toml
N="$2"

if [[ x$STARTERS == "x" || x$N == "x" ]]; then
	echo "Usage: $0 <starter scripts folder> <number of values per proposer>"
    exit 1
fi

# following line kills processes that have the config file in its cmdline
KILLCMD="pkill -f $CONFIG"

$KILLCMD

cd $STARTERS

echo "Generating $N random proposals (which are numbers) for each client..."

../generate.sh $N > ../prop1
../generate.sh $N > ../prop2

echo "Starting (only) 2 acceptors..."

./acceptor.sh 1 $CONFIG &
./acceptor.sh 2 $CONFIG &

sleep 1

echo "Starting 2 learners..."

./learner.sh 1 $CONFIG > ../learn1 &
./learner.sh 2 $CONFIG > ../learn2 &

sleep 1

echo "Starting 2 proposers..."

./proposer.sh 1 $CONFIG &
./proposer.sh 2 $CONFIG &

echo "Waiting 10 seconds before starting clients..."
sleep 10

echo "Starting 2 clients..."

./client.sh 1 $CONFIG < ../prop1 &
./client.sh 2 $CONFIG < ../prop2 &

sleep 5

$KILLCMD
wait

cd ..
