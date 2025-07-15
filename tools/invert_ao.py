"""
JSON Pattern Inverter Script using Python Fire

This script recursively searches through a JSON file to find objects with
name "AmbientOcclusion", inverts the pattern values in their rules
(-1 becomes 1, 1 becomes -1), and saves the modified JSON to a target file.

Usage:
    python script.py process_json input.json output.json
    python script.py process_json input.json output.json --dry_run
"""

import json
import fire
import copy
from typing import Dict, Any, Union, List


class JSONPatternInverter:
    """A class to handle JSON pattern inversion operations."""

    def __init__(self):
        self.found_ambient_occlusion = False
        self.modifications_made = 0

    def _invert_pattern(self, pattern: List[int]) -> List[int]:
        """
        Invert pattern values: -1 becomes 1, 1 becomes -1, 0 stays 0

        Args:
            pattern: List of integers representing the pattern

        Returns:
            List of integers with inverted values
        """
        inverted = []
        for value in pattern:
            if value == -1:
                inverted.append(1)
            elif value == 1:
                inverted.append(-1)
            else:
                inverted.append(value)  # Keep 0 and other values unchanged
        return inverted

    def _process_rules(self, rules: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """
        Process rules array and invert patterns

        Args:
            rules: List of rule dictionaries

        Returns:
            List of modified rule dictionaries
        """
        modified_rules = []

        for rule in rules:
            modified_rule = copy.deepcopy(rule)

            if "pattern" in rule and isinstance(rule["pattern"], list):
                original_pattern = rule["pattern"]
                inverted_pattern = self._invert_pattern(original_pattern)
                modified_rule["pattern"] = inverted_pattern
                self.modifications_made += 1

                print(
                    f"  Rule UID {rule.get('uid', 'unknown')}: "
                    f"Pattern {original_pattern} -> {inverted_pattern}"
                )

            modified_rules.append(modified_rule)

        return modified_rules

    def _search_and_modify(self, obj: Union[Dict, List, Any]) -> Union[Dict, List, Any]:
        """
        Recursively search through JSON structure and modify AmbientOcclusion objects

        Args:
            obj: Current object being processed (dict, list, or primitive)

        Returns:
            Modified object
        """
        if isinstance(obj, dict):
            # Check if this is an AmbientOcclusion object
            if obj.get("name") == "AmbientOcclusion":
                self.found_ambient_occlusion = True
                print(
                    f"Found AmbientOcclusion object with UID: {obj.get('uid', 'unknown')}"
                )

                # Create a copy to avoid modifying the original
                modified_obj = copy.deepcopy(obj)

                # Process rules if they exist
                if "rules" in obj and isinstance(obj["rules"], list):
                    print(f"  Processing {len(obj['rules'])} rules...")
                    modified_obj["rules"] = self._process_rules(obj["rules"])

                return modified_obj
            else:
                # Recursively process all values in the dictionary
                return {
                    key: self._search_and_modify(value) for key, value in obj.items()
                }

        elif isinstance(obj, list):
            # Recursively process all items in the list
            return [self._search_and_modify(item) for item in obj]

        else:
            # Return primitive values unchanged
            return obj

    def process_json(self, input_file: str, output_file: str, dry_run: bool = False):
        """
        Process JSON file to find and modify AmbientOcclusion patterns

        Args:
            input_file: Path to input JSON file
            output_file: Path to output JSON file
            dry_run: If True, don't save the file, just show what would be changed
        """
        try:
            # Reset counters
            self.found_ambient_occlusion = False
            self.modifications_made = 0

            print(f"Loading JSON from: {input_file}")

            # Load the JSON file
            with open(input_file, "r", encoding="utf-8") as f:
                data = json.load(f)

            print("Searching for AmbientOcclusion objects...")

            # Process the JSON structure
            modified_data = self._search_and_modify(data)

            # Report results
            if not self.found_ambient_occlusion:
                print("No AmbientOcclusion objects found in the JSON file.")
                return

            print(f"\nSummary:")
            print(f"  - Found AmbientOcclusion objects: Yes")
            print(f"  - Pattern rules modified: {self.modifications_made}")

            if dry_run:
                print(f"\nDry run mode - no file saved.")
                print(f"Would save to: {output_file}")
            else:
                # Save the modified JSON
                with open(output_file, "w", encoding="utf-8") as f:
                    json.dump(modified_data, f, indent=2, ensure_ascii=False)

                print(f"\nModified JSON saved to: {output_file}")

        except FileNotFoundError:
            print(f"Error: Input file '{input_file}' not found.")
        except json.JSONDecodeError as e:
            print(f"Error: Invalid JSON in input file - {e}")
        except Exception as e:
            print(f"Error: {e}")

    def validate_json(self, file_path: str):
        """
        Validate that a JSON file is properly formatted

        Args:
            file_path: Path to JSON file to validate
        """
        try:
            with open(file_path, "r", encoding="utf-8") as f:
                json.load(f)
            print(f"✓ JSON file '{file_path}' is valid")
        except FileNotFoundError:
            print(f"✗ File '{file_path}' not found")
        except json.JSONDecodeError as e:
            print(f"✗ Invalid JSON in '{file_path}': {e}")
        except Exception as e:
            print(f"✗ Error reading '{file_path}': {e}")


def main():
    """Main entry point for the Fire CLI"""
    fire.Fire(JSONPatternInverter)


if __name__ == "__main__":
    main()
