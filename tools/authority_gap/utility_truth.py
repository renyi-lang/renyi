"""The judge validated on the benchmark's own ground truth: for every user
task of the v1 suites, the ground truth's calls run through AgentDojo's
runtime on the default environment, the environment taken through the
same pickle round trip the utility runs use; a task passes when every call
ran without error and the round trip gives back an equal environment.
The task's `utility` on an empty output is printed as information only:
many utilities read the model's output, which the ground truth has none of.

Usage (in the lane's venv): python utility_truth.py [suite ...]
"""
import pickle
import sys

from agentdojo.functions_runtime import FunctionsRuntime
from agentdojo.task_suite.load_suites import get_suite

SUITES = ["banking", "slack", "travel", "workspace"]


def round_trip(suite, env):
    return pickle.loads(pickle.dumps(env))


def main():
    suites = sys.argv[1:] or SUITES
    for name in suites:
        suite = get_suite("v1", name)
        runtime = FunctionsRuntime(suite.tools)
        passed = failed = utility_true = 0
        for task_id, task in suite.user_tasks.items():
            pre = suite.load_and_inject_default_environment({})
            env = suite.load_and_inject_default_environment({})
            errors = []
            for call in task.ground_truth(pre):
                _, error = runtime.run_function(env, call.function, call.args, raise_on_error=False)
                if error:
                    errors.append(f"{call.function}: {error}")
            post = round_trip(suite, env)
            pre2 = round_trip(suite, pre)
            faithful = post == env and pre2 == pre
            try:
                ok = task.utility("", pre2, post)
            except Exception as error:  # the judge's own failure is a finding, not a crash of the validation
                ok = f"judge raised {type(error).__name__}: {error}"
            utility_true += ok is True
            if faithful and not errors:
                passed += 1
            else:
                failed += 1
                print(f"  {name}/{task_id}: round trip faithful {faithful}; call errors {errors}; utility on empty output {ok}")
        print(f"{name}: {len(suite.user_tasks)} tasks, {passed} faithful without call errors, {failed} not; utility true on empty output {utility_true}")


if __name__ == "__main__":
    main()
