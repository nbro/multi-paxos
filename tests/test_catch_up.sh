#!/usr/bin/env bash

echo "Tests for the 'catch-up' feature required to implement Atomic Broadcast (using Multi-Paxos)."

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

echo "Starting 3 acceptors..."

./acceptor.sh 1 $CONFIG &
./acceptor.sh 2 $CONFIG &
./acceptor.sh 3 $CONFIG &

sleep 1

echo "Starting learner 1..."

./learner.sh 4 $CONFIG > ../learn1 &

sleep 1

echo "Starting 2 proposers..."

./proposer.sh 5 $CONFIG &
./proposer.sh 6 $CONFIG &

echo "Waiting 10 seconds before starting clients..."
sleep 10

echo "Starting client 1..."

./client.sh 7 $CONFIG < ../prop1 &

sleep 1

echo "Starting learner 2..."
./learner.sh 8 $CONFIG > ../learn2 &

sleep 1

echo "Starting client 2..."
./client.sh 9 $CONFIG < ../prop2 &

sleep 5

$KILLCMD
wait

cd ..
