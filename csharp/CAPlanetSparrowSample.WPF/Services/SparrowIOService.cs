using System;
using System.Collections.Generic;
using System.IO;
using System.Text;

using CAPlanetSparrowSample.WPF.Data;

namespace CAPlanetSparrowSample.WPF.Services;

public static class SparrowIOService
{
    public static (bool IsSuccess, SparrowInputData? InputData) InputJsonLoad(string ProjectName, string InputJsonFileFullPath)
    {
        SparrowInputData? _inputData = null;
        try
        {
            _inputData = new()
            {
                ProjectName = ProjectName,
                DirectoryPath = Path.GetDirectoryName(InputJsonFileFullPath)!,
                FileName = Path.GetFileNameWithoutExtension(InputJsonFileFullPath),
                FileExtension = Path.GetExtension(InputJsonFileFullPath).Replace(".", string.Empty)
            };
        }
        catch { }

        return (_inputData is not null, _inputData);
    }
}
