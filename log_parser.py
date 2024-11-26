import json
import os


def parse_log_file(log_file_path, output_dir):
    """
    Parse a log file and extract the 'Offers' list from Push and Read request types.

    :param log_file_path: Path to the log file.
    :param output_dir: Directory where output JSON files will be saved.
    """
    push_offers = []
    read_offers = []
    failed_cases = []
    wanted_cases = []
    actual_cases = []


    # TODO: Parse multiple logs
    try:
        with open(log_file_path, 'r') as file:
            for line in file:
                try:
                    log_entry = json.loads(line)

                    request_type = log_entry.get('requestType', '').lower()
                    if request_type == 'push':
                        push_offers += (log_entry.get('log', [])["write_config"]["Offers"])

                    elif 'read' in request_type:
                        entry = log_entry.get('log', [])
                        read_offers.append(entry['search_config'])
                        if "expected_result" not in entry:
                            continue
                        wanted_cases.append(entry['expected_result'])
                        if "actual_result" not in entry:
                            continue
                        actual_cases.append(entry['actual_result'])
                        wanted = entry['expected_result']
                        actual = entry['actual_result']
                        if wanted != actual:
                            diff = {"input":[], "diff":[]}
                            diff['input'].append(entry['search_config'])
                            temp = []
                            for key in wanted:
                                if wanted[key] != actual[key]:
                                    diff_dic = {"actual_"+key: actual[key], "wanted_"+key: wanted[key]}
                                    if key == "Offers":
                                        diff_dic["actual_amount"] = len(actual[key])
                                        diff_dic["wanted_amount"] = len(wanted[key])
                                    temp.append(diff_dic)

                            diff['diff'] += temp
                            failed_cases.append(diff)


                except json.JSONDecodeError:
                    continue

        os.makedirs(output_dir, exist_ok=True)

        push_file = os.path.join(output_dir, "push_offers.json")
        read_file = os.path.join(output_dir, "read_offers.json")
        failed_file = os.path.join(output_dir, "failed_cases.json")
        wanted_file = os.path.join(output_dir, "wanted.json")
        actual_file = os.path.join(output_dir, "actual.json")

        with open(push_file, 'w') as f:
            json.dump(push_offers, f, indent=4)
        with open(read_file, 'w') as f:
            json.dump(read_offers, f, indent=4)
        with open(failed_file, 'w') as f:
            json.dump(failed_cases, f, indent=4)
        with open(wanted_file, 'w') as f:
            json.dump(wanted_cases, f, indent=4)
        with open(actual_file, 'w') as f:
            json.dump(actual_cases, f, indent=4)

    except FileNotFoundError:
        print(f"Error: The file {log_file_path} was not found.")
    #except Exception as e:
     #   print(f"An unexpected error occurred: {e}")

log_file_path = "logs/test.log"
output_dir = "logs/test_inputs"
parse_log_file(log_file_path, output_dir)
