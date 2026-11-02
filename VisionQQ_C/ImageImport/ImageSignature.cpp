#include "pch.h"
#include "ImageSignature.h"


std::expected<std::wstring, std::exception> getRealExtension(const wchar_t* src)
{
	std::ifstream file(src, std::ios::binary);
	if (!file.is_open())
	{
		return std::unexpected(std::invalid_argument("Cannot open file"));
	}

	std::vector<uint8_t> header(12);
	file.read(reinterpret_cast<char*>(header.data()), header.size());
	size_t bytesRead = file.gcount();
	file.close();

	if (bytesRead < 2)
	{
		return std::unexpected(std::range_error("File is too small"));
	}
	for (const ImgSignature& imgSignature : SIGNATURES)
	{
		if (bytesRead >= imgSignature.magic.size())
		{
			bool match = true;
			for (size_t i = 0; i < imgSignature.magic.size(); ++i)
			{
				if (header[i] != imgSignature.magic[i])
				{
					match = false;
					break;
				}
			}
			if (match)
			{
				if (imgSignature.format == std::wstring(L"RIFF") and bytesRead >= 12)
				{
					if (header[8] == 'W' and header[9] == 'E' and header[10] == 'B' and header[11] == 'P')
					{
						return L"webp";
					}
					else
					{
						return L"avi";
					}
				}
				else
				{
					return imgSignature.ext;
				}
			}
		}

	}
	return std::unexpected(std::out_of_range("Unknown ext"));

}