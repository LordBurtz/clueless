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
                        read_offers.append(log_entry.get('log', [])['search_config'])
                        wanted_cases.append(log_entry.get('log', [])['expected_result'])
                        actual_cases.append(log_entry.get('log', [])['actual_result'])
                        wanted = log_entry.get('log', [])['expected_result']
                        actual = log_entry.get('log', [])['actual_result']
                        if wanted != actual:
                            failed_cases.append(log_entry.get('log', [])['search_config'])
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
    except Exception as e:
        print(f"An unexpected error occurred: {e}")

log_file_path = "logs/test.log"
output_dir = "logs/test_inputs"
parse_log_file(log_file_path, output_dir)
