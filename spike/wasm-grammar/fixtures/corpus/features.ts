import { readFile } from "node:fs/promises";
import type { Stats } from "node:fs";

export interface Point<T = number> {
  readonly x: T;
  y: T;
  label?: string;
}

export type Shape =
  | { kind: "circle"; radius: number }
  | { kind: "square"; size: number };

export enum Direction {
  North = "north",
  South = "south",
}

export abstract class Base<T extends object> {
  protected constructor(public readonly value: T) {}

  abstract describe(): string;
}

export class Box<T> extends Base<{ item: T }> {
  #count = 0;

  constructor(item: T) {
    super({ item });
  }

  describe(): string {
    return `box(${this.value.item}) #${this.#count++}`;
  }
}

export async function loadStats(path: string): Promise<Stats> {
  const data = await readFile(path);
  return data.length > 0 ? { size: data.length } : ({} as Stats);
}

export const area = (shape: Shape): number =>
  shape.kind === "circle" ? Math.PI * shape.radius ** 2 : shape.size ** 2;

export function isCircle(
  shape: Shape,
): shape is Extract<Shape, { kind: "circle" }> {
  return shape.kind === "circle";
}

const map = new Map<string, Point<number>>();
map.set("origin", { x: 0, y: 0 });

function logged<T extends (...args: never[]) => unknown>(fn: T): T {
  return ((...args: never[]) => fn(...args)) as T;
}

export class Service {
  @logged
  run(): void {}
}

export default function distance(a: Point, b: Point): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}
