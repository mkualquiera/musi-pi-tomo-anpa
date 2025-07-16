import json
from PIL import Image
import numpy as np


def extract_sprites_and_edges(image_path, output_json_path, output_rust_path=None):
    # Load the image
    img = Image.open(image_path).convert("RGBA")
    img_array = np.array(img)

    # Constants
    SPRITE_SIZE = 16
    COLS = 11
    ROWS = 5

    results = {}
    counter = 0

    for col in range(COLS):
        for row in range(ROWS):
            # Extract sprite bounds
            x_start = col * SPRITE_SIZE
            y_start = row * SPRITE_SIZE
            x_end = x_start + SPRITE_SIZE
            y_end = y_start + SPRITE_SIZE

            # Extract sprite
            sprite = img_array[y_start:y_end, x_start:x_end]

            # Check if top-left pixel is fully red (discard if so)
            top_left_pixel = sprite[0, 0]
            if (
                top_left_pixel[0] > 250
                and top_left_pixel[1] < 5
                and top_left_pixel[2] < 5
            ):  # Roughly red
                print(f"Discarding sprite at ({col}, {row}) - red pixel detected")
                continue

            # Sample edges in 3x3 grid pattern
            # Each sample point is at (0, 1, 2) * 8 = (0, 8, 15) on each axis
            edge_matrix = []

            for grid_y in range(3):
                row_data = []
                for grid_x in range(3):
                    # Sample coordinates
                    sample_x = grid_x * 8
                    sample_y = grid_y * 8

                    # Clamp to sprite bounds (should be 0-15)
                    sample_x = min(sample_x, SPRITE_SIZE - 1)
                    sample_y = min(sample_y, SPRITE_SIZE - 1)

                    # Get pixel and check if it's black (r < 0.03)
                    pixel = sprite[sample_y, sample_x]
                    r_value = pixel[0] / 255.0  # Normalize to 0-1

                    is_black = 1.0 if r_value < 0.03 else 0.0
                    row_data.append(is_black)

                # Pad each row to 4 elements for JSON (mat4)
                row_data.append(0.0)  # Padding for mat4
                edge_matrix.extend(row_data)

            # Add final padding row (4 zeros) for JSON
            edge_matrix.extend([0.0, 0.0, 0.0, 0.0])

            results[f"{counter}"] = edge_matrix
            counter += 1
            print(f"Processed sprite at ({col}, {row})")

    # Save results as JSON
    with open(output_json_path, "w") as f:
        json.dump(results, f, indent=2)

    # Save results as Rust file if path provided
    if output_rust_path:
        write_rust_file(results, output_rust_path)

    print(f"Extracted {len(results)} sprites and saved to {output_json_path}")
    if output_rust_path:
        print(f"Also saved Rust file to {output_rust_path}")


def write_rust_file(results, output_path):
    """Write the adjacency rules to a Rust file without padding"""
    with open(output_path, "w") as f:
        f.write("const ADJACENCY_RULES: &[&[f32]] = &[\n")

        # Sort by numeric key to maintain order
        sorted_keys = sorted(results.keys(), key=lambda x: int(x))

        for i, key in enumerate(sorted_keys):
            padded_matrix = results[key]

            # Remove padding: extract 3x3 grid from the padded 4x4 matrix
            unpadded_matrix = []
            for row in range(3):
                start_idx = row * 4  # Each padded row has 4 elements
                # Take only the first 3 elements from each row
                unpadded_matrix.extend(padded_matrix[start_idx : start_idx + 3])

            # Format as Rust array
            formatted_values = [f"{val:.1f}" for val in unpadded_matrix]
            f.write(f"    &[{', '.join(formatted_values)}]")

            # Add comma if not the last item
            if i < len(sorted_keys) - 1:
                f.write(",")
            f.write("\n")

        f.write("];\n")


# Usage
if __name__ == "__main__":
    # extract_sprites_and_edges("spritesheet.png", "sprite_edges.json", "adjacency_rules.rs")
    import fire

    fire.Fire(extract_sprites_and_edges)
