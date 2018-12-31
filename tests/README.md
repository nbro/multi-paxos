# Tests for the Atomic Broadcast implementation using Multi-Paxos

These are some basic tests that can be used to check the correctness and features of this Multi-Paxos implementation of Atomic Broadcast. They are not comprehensive but should give us enough confidence this implementation works (or not).

## Dependencies

This tests are use basic Unix tools such as `grep`, `sed`, etc. Furthermore, if you want to run the test with message losses, you need `iptables` with `sudo` access.

Depending on the network interface used, IP multicast might not be enabled. You can check using `ifconfig` and checking that the
`MULTICAST` flag is set. You *might* have to enable it. Check out on the web how to enable it for your system. Using a connected cable/wifi interface (e.g. `eth0`, `wlan0`) will probably not have this problem.

## How to test this Atomic Broadcast implementation?

Under the [`tests`](./) folder (containing this README file that you are reading), there are several tests that can be run. Below, it is briefly described what each test does and how you can run it. Before proceeding, make sure you are inside this [`tests`](./) folder.

1. You can test that this Atomic Broadcast implementation (using Multi-Paxos) works when all learners, proposers and acceptors are already running and listening to messages, when the clients start (one immediately after the other). In this case, there are 3 acceptors, 2 proposers, 2 learners and 2 clients. Each client can propose n values, where n is passed as input by the user which executes the test. Please, have a look at the script [`test_basic.sh`](./test_basic.sh) for more info. To run this test, you can execute the following command
    
       ./test_basic.sh starters 100 && ./check_all.sh
    
    where [`starters`](./tests/starters) is the folder (under [`tests`](./tests)) containing the Bash scripts used to run the clients, proposers, acceptors and learners and 100 is the number of proposals that the client will propose. Of course, you can and should also try the tests with a different number of proposals (e.g. 10 or 50): note that you should not try very big numbers, as there is a limit regarding how many bytes can be sent using the UDP sockets, which back my implementation up. 
    
    "Test 3" might fail in some cases, but with few proposed values and no message loss it should succeed.

2. You can test the "catch-up" feature of the learners, that is, the feature that allows the learners, which are instantiated after other basic Paxos instances have already been run, to "catch-up" the state of affairs of the proposers (i.e., to know about the previously learned values): this is basically the feature which is required to implement AB using Multi-Paxos. In this case, at the end, we will also have 3 acceptors, 2 proposers, 2 learners and 2 clients, but we will start them in a different order. First, all 3 acceptors are started. Then we start only 1 learner, and then 2 proposers. We then start only 1 client, which starts to propose values. Meanwhile, the only currently executing learner should be able to learn the values proposed by this client. Then we start the second learner, which should "catch up" the first learner and print the values that the first learner already learned during the previously executed Paxos instances. Finally, we start the second client, which will propose more values, and both learners should be able to learn them. Please, have a look at the script [`test_catch_up.sh`](./test_catch_up.sh) for more info. You can run this test by issuing the following command on the terminal

       ./test_catch_up.sh starters 100 && ./check_all.sh

3. You can also test how this Atomic Broadcast implementation works in the case there are only 2 acceptors (and still 2 proposers, 2 learners, and 2 clients). For more info regarding this test, have a look at the script [`test_2acceptors.sh`](./test_2acceptors.sh). You run this test by issuing the command

       ./test_2acceptors.sh starters 100 && ./check_all.sh

4. You can also test it when there is only 1 acceptor. Have a look at the file [`test_1acceptor.sh`](./test_1acceptor.sh) for more info. Note that Paxos and Multi-Paxos require that the acceptors are more than 1 in order to tolerate the failure of f acceptors. You can run this test as follows

       ./test_1acceptor.sh starters 100 && ./check_all.sh
       
   In this case, the learners should not be able to learn anything, so "Test 3" should fail.
    
5. You can also test how the implementation deals with the situation of message loss by issuing the command

       ./test_loss.sh starters 100 && ./check_all.sh
       
     
## Caveats, Tips and Notes

1. The scripts will try to "kill" your processes (SIGTERM). You might need to "flush" the output of your learners to make sure values are printed when learned.

2. The output of your "learners" should be ONLY the values learned, one per line. Anything else will fail the checks.

3. The scripts wait some seconds after starting the different processes, to be sure there is some time for your implementation to "stabilize". 

4. The scripts wait for around 5 seconds after starting the clients for values to be learned. Depending on the amount of values proposed and your implementation, it might not be enough. For around 100-1000 values per client it should be enough time.

5. The [`test_loss.sh`](./test_loss.sh) script uses a 10% loss probability which you can
change inside the script. It does it by adding a rule/filter with `iptables`.  It will also remove it using [`loss_unset.sh`](./loss_unset.sh). If kill the script for some reason, you might have to call `loss_unset.sh` by hand to remove the filter.