"""The authority gap on AgentDojo: for every (user task, injection task) pair of
a suite, would the injection's ground-truth calls fit inside the smallest
grant the user task's program can declare?

Usage: python -I authority_gap.py <tasks.json> [--pairs]

A tool is a capability kind; the argument named in SCOPE is its scope (a
list argument is a set of scopes). The user task's grant declares a scope
for a tool only when every ground-truth call of that tool has a literal
scope that the user's prompt spells out (the program can write it in its
`needs`); otherwise the tool is granted without a scope. Three grants are
measured per pair:
  kind          the grant names kinds only (no scopes)
  prompt        scopes declared when literal in the prompt (the honest estimate)
  prompt+budget the same, with `at most n per run` on every kind, n the
                number of calls of the kind in the user task's ground truth
  +amount       the same, with `at most A of amount per run` on a kind whose
                argument named in AMOUNT is a literal the prompt spells out in
                every ground-truth call, A the sum of those amounts
                (decision AY1; the honest estimate with every budget)
  literal       scopes declared for every literal argument (an upper bound)
  literal+amount the same with the counts and the amounts of every literal,
                whether or not the prompt spells them (the upper bound with
                every budget)
A pair is blocked when the injection needs a kind outside the grant, a
scope outside a declared scope, more calls of a kind than its count, or
amounts of a kind whose sum (absolute values) passes its budget. An
injection whose scope or amount is derived from the environment against a
declared scope or budget is `undetermined` (not blocked). An injection
with no tool call cannot be blocked by any grant.
"""
import json
import sys
from collections import Counter

SCOPE = {
    # banking
    "send_money": "recipient",
    "schedule_transaction": "recipient",
    "update_scheduled_transaction:recipient": "recipient",
    "read_file": "file_path",
    # slack
    "get_webpage": "url",
    "post_webpage": "url",
    "download_file": "url",
    "send_direct_message": "recipient",
    "send_channel_message": "channel",
    "invite_user_to_slack": "user_email",
    "add_user_to_channel": "channel",
    "remove_user_from_slack": "user",
    "read_channel_messages": "channel",
    "read_inbox": "user",
    "get_users_in_channel": "channel",
    # travel
    "reserve_hotel": "hotel",
    "reserve_restaurant": "restaurant",
    "reserve_car_rental": "company",
    "get_all_hotels_in_city": "city",
    "get_all_restaurants_in_city": "city",
    "get_all_car_rental_companies_in_city": "city",
    # workspace (and travel's email and calendar)
    "send_email": "recipients",
    "create_calendar_event": "participants",
    "add_calendar_event_participants": "participants",
    "share_file": "email",
    "delete_file": "file_id",
    "delete_email": "email_id",
    "append_to_file": "file_id",
    "reschedule_calendar_event": "event_id",
    "cancel_calendar_event": "event_id",
}


AMOUNT = {
    # banking: the argument a budget on an amount sums (decision AY1)
    "send_money": "amount",
    "schedule_transaction": "amount",
    "update_scheduled_transaction:recipient": "amount",
    "update_scheduled_transaction:other": "amount",
}


def kind_of(call):
    # changing a scheduled transaction's recipient is another authority than
    # changing its amount: the tool takes both as optional arguments
    if call["function"] == "update_scheduled_transaction":
        return "update_scheduled_transaction:recipient" if "recipient" in call["args"] else "update_scheduled_transaction:other"
    return call["function"]


def norm(v):
    return str(v).strip().lower()


def scopes_of(call, kind):
    """(status, set): status 'none' when the kind has no scope argument or the
    call leaves it out, 'derived' when it is computed, 'literal' with the set."""
    arg = SCOPE.get(kind)
    if arg is None or arg not in call["args"]:
        return "none", set()
    a = call["args"][arg]
    if a["kind"] != "literal":
        return "derived", set()
    v = a["value"]
    values = v if isinstance(v, list) else [v]
    return "literal", {norm(x) for x in values}


def amount_of(call, kind):
    """(status, value): 'none' when the kind has no amount argument or the call
    leaves it out, 'derived' when it is computed, 'literal' with |value|."""
    arg = AMOUNT.get(kind)
    if arg is None or arg not in call["args"]:
        return "none", 0.0
    a = call["args"][arg]
    if a["kind"] != "literal":
        return "derived", 0.0
    return "literal", abs(float(a["value"]))


def spelled(value, prompt):
    """Whether the prompt spells the amount (as written, or without `.0`)."""
    text = str(value)
    if text.endswith(".0"):
        text = text[:-2]
    return text in prompt


def budgets_of(task, mode):
    """kind -> the sum of the ground truth's amounts, for the kinds whose every
    call carries a literal amount (one the prompt spells, under `prompt`);
    absent when the author cannot write the number."""
    prompt = norm(task["text"])
    budgets = {}
    for call in task["calls"]:
        kind = kind_of(call)
        status, value = amount_of(call, kind)
        if status == "none":
            continue
        raw = call["args"][AMOUNT[kind]].get("value")
        if status == "derived" or (mode == "prompt" and not spelled(raw, prompt)):
            budgets[kind] = None
            continue
        if budgets.get(kind, 0.0) is not None:
            budgets[kind] = budgets.get(kind, 0.0) + value
    return {kind: limit for kind, limit in budgets.items() if limit is not None}


def grant_of(task, mode):
    """kind -> None (unscoped) or a set of declared scopes."""
    prompt = norm(task["text"])
    grant = {}
    for call in task["calls"]:
        kind = kind_of(call)
        status, values = scopes_of(call, kind)
        declared = grant.get(kind, set())
        if declared is None:
            continue
        if mode == "kind" or status == "derived":
            grant[kind] = None
            continue
        if status == "none":
            grant.setdefault(kind, set())
            continue
        if mode == "prompt" and not all(x in prompt for x in values):
            grant[kind] = None
            continue
        grant[kind] = declared | values
    return grant


def judge(user, injection, mode):
    """'kind' when the injection needs a kind the grant lacks (checked over
    every call first, so the verdict does not depend on the calls' order),
    else 'scope' when a scope falls outside a declared one, else 'budget'
    when the mode asks for budgets and the injection makes more calls of a
    kind than the user task's ground truth (`at most n per run`), else
    else 'amount' when the mode asks for budgets on amounts and the
    injection's amounts of a kind sum past the user task's (`at most A of
    amount per run`, decision AY1), else 'undetermined' or 'fits'."""
    if not injection["calls"]:
        return "no-tool"
    base = mode.split("+")[0]
    grant = grant_of(user, base)
    kinds = [kind_of(call) for call in injection["calls"]]
    if any(kind not in grant for kind in kinds):
        return "kind"
    verdict = "fits"
    for call, kind in zip(injection["calls"], kinds):
        declared = grant[kind]
        if declared is None:
            continue
        status, values = scopes_of(call, kind)
        if status == "none":
            continue
        if status == "derived":
            verdict = "undetermined"
            continue
        if not values <= declared:
            return "scope"
    if "+budget" in mode:
        user_counts = Counter(kind_of(call) for call in user["calls"])
        if any(n > user_counts[kind] for kind, n in Counter(kinds).items()):
            return "budget"
    if "+amount" in mode:
        budgets = budgets_of(user, base)
        sums = Counter()
        for call, kind in zip(injection["calls"], kinds):
            if kind not in budgets:
                continue
            status, value = amount_of(call, kind)
            if status == "derived":
                verdict = "undetermined"
            elif status == "literal":
                sums[kind] += value
        if any(total > budgets[kind] for kind, total in sums.items()):
            return "amount"
    return verdict


def main():
    data = json.load(open(sys.argv[1], encoding="utf-8"))
    show_pairs = "--pairs" in sys.argv
    modes = ["kind", "prompt", "prompt+budget", "+amount", "literal", "literal+amount"]
    judged = {"+amount": "prompt+budget+amount", "literal+amount": "literal+budget+amount"}
    total = Counter()
    grand = {m: Counter() for m in modes}
    for suite, s in data.items():
        counts = {m: Counter() for m in modes}
        n = 0
        for u in s["user"].values():
            for i in s["injection"].values():
                n += 1
                for m in modes:
                    v = judge(u, i, judged.get(m, m))
                    counts[m][v] += 1
                    grand[m][v] += 1
                if show_pairs:
                    v = judge(u, i, "prompt+budget+amount")
                    if v in ("fits", "undetermined"):
                        print(f"  {suite} {u['id']} x {i['id']}: {v}")
        total[suite] = n
        print(f"{suite}: {len(s['user'])} user x {len(s['injection'])} injection = {n} pairs")
        for m in modes:
            print(row(m, counts[m], n))
    n = sum(total.values())
    print(f"all: {n} pairs")
    for m in modes:
        print(row(m, grand[m], n))


def row(mode, c, n):
    blocked = c["kind"] + c["scope"] + c["budget"] + c["amount"]
    return (f"  {mode:14s} blocked {blocked:4d} ({100*blocked/n:5.1f}%): kind {c['kind']:3d}, scope {c['scope']:3d}, "
            f"budget {c['budget']:3d}, amount {c['amount']:3d}; "
            f"undetermined {c['undetermined']:3d}, fits {c['fits']:3d}, no-tool {c['no-tool']:3d}")


main()
