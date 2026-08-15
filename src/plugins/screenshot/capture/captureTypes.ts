export type CaptureTool =
  | "select"
  | "rectangle"
  | "ellipse"
  | "arrow"
  | "pen"
  | "text"
  | "mosaic"
  | "pin";

export interface Point {
  x: number;
  y: number;
}

interface BaseAnnotation {
  id: string;
  color: string;
  strokeWidth: number;
}

export interface RectangleAnnotation extends BaseAnnotation {
  type: "rectangle";
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface EllipseAnnotation extends BaseAnnotation {
  type: "ellipse";
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface ArrowAnnotation extends BaseAnnotation {
  type: "arrow";
  from: Point;
  to: Point;
}

export interface PenAnnotation extends BaseAnnotation {
  type: "pen";
  points: Point[];
}

export interface TextAnnotation extends BaseAnnotation {
  type: "text";
  x: number;
  y: number;
  text: string;
  fontSize: number;
}

export interface MosaicAnnotation extends BaseAnnotation {
  type: "mosaic";
  x: number;
  y: number;
  width: number;
  height: number;
  pixelSize: number;
}

export interface PinAnnotation extends BaseAnnotation {
  type: "pin";
  x: number;
  y: number;
  width: number;
  height: number;
}

export type AnnotationObject =
  | RectangleAnnotation
  | EllipseAnnotation
  | ArrowAnnotation
  | PenAnnotation
  | TextAnnotation
  | MosaicAnnotation
  | PinAnnotation;

export interface CaptureSession {
  session_id: string;
  image_base64: string;
  initial_action: "copy" | "save";
  width: number;
  height: number;
}

export interface HistoryState {
  annotations: AnnotationObject[];
  selectedId: string | null;
  undoStack: AnnotationObject[][];
  redoStack: AnnotationObject[][];
}
