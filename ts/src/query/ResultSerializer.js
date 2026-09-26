import { JsonArray, JsonObject } from "com.google.gson";

export class ResultSerializer {
  static row(metadata, resultSet) {
    const row = new JsonObject();

    for (let column = 1; column <= metadata.getColumnCount(); column++) {
      const label = metadata.getColumnLabel(column);
      const value = resultSet.getString(column);

      if (row.has(label)) {
        row.addProperty(`${label}_${column}`, value);
      } else {
        row.addProperty(label, value);
      }
    }

    return row;
  }

  static toResponse(result) {
    const response = new JsonObject();
    const rows = new JsonArray();

    result.rows.forEach((row) => rows.add(row));
    response.add("rows", rows);
    response.addProperty("rowsUpdated", result.rowsUpdated);
    response.addProperty("elapsedMs", result.elapsedMs);
    response.addProperty("success", true);

    return response;
  }
}