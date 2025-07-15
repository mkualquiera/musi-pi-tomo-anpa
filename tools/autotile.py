import json
from PIL import Image
import numpy as np


def extract_sprites_and_edges(image_path, output_json_path):
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

                # Pad each row to 4 elements
                row_data.append(0.0)  # Padding for mat4
                edge_matrix.extend(row_data)

            # Add final padding row (4 zeros)
            edge_matrix.extend([0.0, 0.0, 0.0, 0.0])

            # results.append(edge_matrix)
            results[f"{counter}"] = edge_matrix
            counter += 1
            print(f"Processed sprite at ({col}, {row})")

    # Save results as JSON
    with open(output_json_path, "w") as f:
        json.dump(results, f, indent=2)

    print(f"Extracted {len(results)} sprites and saved to {output_json_path}")


# Usage
if __name__ == "__main__":
    # extract_sprites_and_edges("spritesheet.png", "sprite_edges.json")
    import fire

    fire.Fire(extract_sprites_and_edges)
