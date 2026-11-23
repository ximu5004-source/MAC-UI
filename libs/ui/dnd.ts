import type { Plugins, Sensors } from "@dnd-kit/abstract";
import {
  DragDropManager,
  // KeyboardSensor,
  PointerActivationConstraints,
  PointerSensor,
} from "@dnd-kit/dom";

function DND_PLUGINS(defaults: Plugins): Plugins {
  return [...defaults];
}

export function createDragDropSensors(isDragAllowed: () => boolean = () => true): Sensors {
  return [
    PointerSensor.configure({
      // Allow dragging from buttons and other interactive elements inside items.
      preventActivation: () => !isDragAllowed(),
      // Prevents unintentional drags on click by requiring a 5px movement before activation.
      activationConstraints: (event, _source) => {
        const { pointerType } = event;
        switch (pointerType) {
          case "mouse":
            return [new PointerActivationConstraints.Distance({ value: 5 })];
          case "touch":
            return [new PointerActivationConstraints.Delay({ value: 250, tolerance: 5 })];
          default:
            return [
              new PointerActivationConstraints.Delay({ value: 200, tolerance: 10 }),
              new PointerActivationConstraints.Distance({ value: 5 }),
            ];
        }
      },
    }),
    /* KeyboardSensor.configure({
    preventActivation: () => true,
  }), */
  ];
}

export function createDragDropManager(isDragAllowed: () => boolean = () => true) {
  return new DragDropManager({
    sensors: createDragDropSensors(isDragAllowed),
    plugins: DND_PLUGINS,
  });
}
