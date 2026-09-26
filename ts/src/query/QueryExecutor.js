import "br.com.sankhya.modelcore.util.EntityFacadeFactory";
import "br.com.sankhya.jape.dao.JdbcWrapper";
import "com.sankhya.util.JdbcUtils";
import { ResultSerializer } from "./ResultSerializer";

export default class QueryExecutor {
  execute(query) {
    let jdbc = null;
    let resultSet = null;
    const startedAt = System.currentTimeMillis();

    try {
      jdbc = EntityFacadeFactory.getDWFFacade().getJdbcWrapper();
      jdbc.openSession();

      const statement = jdbc.getPreparedStatement(query);
      const hasRows = statement.execute();
      const rows = [];
      let rowsUpdated = statement.getUpdateCount();

      if (hasRows) {
        resultSet = statement.getResultSet();
        const metadata = resultSet.getMetaData();

        while (resultSet.next()) {
          rows.push(ResultSerializer.row(metadata, resultSet));
        }
        rowsUpdated = rows.length;
      }

      return {
        rows,
        rowsUpdated,
        elapsedMs: System.currentTimeMillis() - startedAt,
      };
    } catch (error) {
      throw new Error(`Erro ao executar consulta: ${error}`);
    } finally {
      JdbcUtils.closeResultSet(resultSet);
      JdbcWrapper.closeSession(jdbc);
    }
  }
}