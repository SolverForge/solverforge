#!/bin/bash

# Mapping of old commit messages to new conventional commit messages
case "$GIT_COMMIT" in
    ff6c68af57cd14f7dbf6d6391042a4d0bad2a603)
        echo "refactor(solver): extract list_ruin.rs tests to selector/tests/list_ruin.rs"
        ;;
    c2f8428e5147b183df998d9049caffc7ca07593d)
        echo "refactor(solver): extract pillar.rs tests to selector/tests/pillar.rs"
        ;;
    e100b4df8d15182364fb5a3fb726dc203a56f1dd)
        echo "refactor(solver): remove duplicate k_opt_reconnection tests, keep internal invariant tests"
        ;;
    580dccebc72d96fe4bb7a0585fe10eaa4417bd94)
        echo "refactor(solver): restructure heuristic/move/tests.rs into tests/ subdirectory"
        ;;
    306b3723d0a27a939f5535d72c4a934e22e9b752)
        echo "fix(docs): correct inappropriate /// doc comments on private items"
        ;;
    *)
        cat
        ;;
esac
