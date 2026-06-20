pub struct Topic {
    pub name: &'static str,
    pub desc: &'static str,
    pub time: &'static str,
    pub space: &'static str,
    pub stable: bool,
    pub gen_steps: fn(&[u32]) -> Vec<super::Step>,
}

pub const TOPICS: &[Topic] = &[
    Topic {
        name: "Bubble Sort",
        desc: "Repeatedly steps through the list, compares adjacent elements and swaps them if they are in the wrong order.",
        time: "O(n²)",
        space: "O(1)",
        stable: true,
        gen_steps: super::sorting::bubble_sort_steps,
    },
    Topic {
        name: "Selection Sort",
        desc: "Divides the input into a sorted and unsorted region, repeatedly selecting the smallest element from the unsorted region.",
        time: "O(n²)",
        space: "O(1)",
        stable: false,
        gen_steps: super::sorting::selection_sort_steps,
    },
    Topic {
        name: "Insertion Sort",
        desc: "Builds the final sorted array one element at a time by repeatedly inserting an element into its correct position.",
        time: "O(n²)",
        space: "O(1)",
        stable: true,
        gen_steps: super::sorting::insertion_sort_steps,
    },
    Topic {
        name: "Merge Sort",
        desc: "Divides the array into halves, recursively sorts each half, then merges the sorted halves back together.",
        time: "O(n log n)",
        space: "O(n)",
        stable: true,
        gen_steps: super::sorting::merge_sort_steps,
    },
    Topic {
        name: "Quick Sort",
        desc: "Selects a pivot, partitions the array around it, and recursively sorts the partitions.",
        time: "O(n log n)",
        space: "O(log n)",
        stable: false,
        gen_steps: super::sorting::quick_sort_steps,
    },
    Topic {
        name: "Heap Sort",
        desc: "Builds a max-heap from the array, then repeatedly extracts the maximum element to build the sorted list.",
        time: "O(n log n)",
        space: "O(1)",
        stable: false,
        gen_steps: super::sorting::heap_sort_steps,
    },
];
